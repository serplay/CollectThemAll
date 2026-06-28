//! MapGenie integration module.
//!
//! Cybersecurity studies note (1st year, 2nd semester): a big single file is hard
//! to audit. For this assignment we are splitting the old 900-line file into smaller
//! modules so each piece does one job — this is the "separation of concerns" idea
//! from our software security lectures. Smaller files = easier to read = easier to
//! spot mistakes.
//!
//! For now this file (`mod.rs`) is still the main entry point; we move things out
//! into sibling modules one step at a time.

use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs;

// The data "shapes" now live in their own file. We pull them back in here so the
// rest of the code can keep using them by their short names (Game, Map, ...).
mod models;
pub use models::*;

// The scraped tile-configuration structs and their custom deserializer.
mod tile_config;
use tile_config::*;

/// Max simultaneous tile downloads. Tiles are tiny so we can fan out fairly wide
/// without overwhelming the CDN; this is the main lever for download speed.
const TILE_DOWNLOAD_CONCURRENCY: usize = 16;

/// Shared HTTP client for on-demand tile fetches by the protocol handler.
static TILE_CLIENT: OnceLock<Client> = OnceLock::new();

/// In-memory cache of per-map tile metadata (zoom range, extension, CDN URL template)
/// so the protocol handler doesn't re-read/parse tile_meta.json on every tile request.
static TILE_META_CACHE: OnceLock<tokio::sync::Mutex<HashMap<(u32, u32), TileMeta>>> =
    OnceLock::new();

fn tile_client() -> &'static Client {
    TILE_CLIENT.get_or_init(|| build_client().expect("failed to build tile HTTP client"))
}

// --- Helper Functions ---

fn build_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())
}

async fn get_cached_game(app: &AppHandle, game_id: u32) -> Result<Game, String> {
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let cache_file = cache_dir.join("mapgenie_games_cache.json");
    let cache_content = fs::read_to_string(&cache_file)
        .await
        .map_err(|_| "Game cache not found — fetch the games list first".to_string())?;
    let cache_data: CacheData = serde_json::from_str(&cache_content).map_err(|e| e.to_string())?;
    cache_data
        .games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("Game with id {} not found in cache", game_id))
}

/// Extracts the JSON value of a named JavaScript variable from raw HTML.
/// Handles both `window.VAR = {...}` and `const VAR = {...}` patterns.
/// Uses serde_json's streaming deserializer so it reads exactly one valid JSON
/// value and stops, safely ignoring the trailing semicolon/script content.
fn extract_named_var_json(html: &str, prefix: &str) -> Result<serde_json::Value, String> {
    let start = html
        .find(prefix)
        .ok_or_else(|| format!("'{}' not found in page", prefix.trim()))?
        + prefix.len();
    let slice = &html[start..];
    serde_json::Deserializer::from_str(slice)
        .into_iter::<serde_json::Value>()
        .next()
        .ok_or_else(|| "Failed to parse JSON after variable prefix".to_string())?
        .map_err(|e| e.to_string())
}

async fn fetch_map_tile_config(
    client: &Client,
    game_slug: &str,
    map_slug: &str,
) -> Result<(MapConfig, serde_json::Value), String> {
    let url = format!("https://mapgenie.io/{}/maps/{}", game_slug, map_slug);
    let html = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let map_data_json = extract_named_var_json(&html, "window.mapData = ")?;
    let parsed: MapDataHtml =
        serde_json::from_value(map_data_json.clone()).map_err(|e| e.to_string())?;

    // Also return the raw mapData JSON so callers can extract other fields (e.g. MARKER_SPRITE_POSITIONS_V3)
    let marker_positions = extract_named_var_json(&html, "const MARKER_SPRITE_POSITIONS_V3 = ")
        .unwrap_or(serde_json::Value::Null);

    Ok((parsed.map_config, marker_positions))
}

// --- Commands ---

#[tauri::command]
pub async fn fetch_and_cache_games(app: AppHandle) -> Result<Vec<Game>, String> {
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let cache_file = cache_dir.join("mapgenie_games_cache.json");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let twelve_hours = 12 * 60 * 60;

    if let Ok(cache_content) = fs::read_to_string(&cache_file).await {
        if let Ok(cache_data) = serde_json::from_str::<CacheData>(&cache_content) {
            if now - cache_data.timestamp < twelve_hours {
                return Ok(cache_data.games);
            }
        }
    }

    let client = build_client()?;

    let response = client
        .get("https://mapgenie.io/api/v1/games")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let games: Vec<Game> = response.json().await.map_err(|e| e.to_string())?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let assets_dir = data_dir.join("assets");

    for game in &games {
        let game_dir = assets_dir.join(game.id.to_string());
        let _ = fs::create_dir_all(&game_dir).await;

        if let Some(img_url) = &game.image {
            let _ = download_file(&client, img_url, game_dir.join("image.jpg")).await;
        }
        if let Some(logo_url) = &game.logo {
            let _ = download_file(&client, logo_url, game_dir.join("logo.jpg")).await;
        }
    }

    let cache_data = CacheData {
        timestamp: now,
        games: games.clone(),
    };
    if let Ok(cache_json) = serde_json::to_string(&cache_data) {
        let _ = fs::create_dir_all(&cache_dir).await;
        let _ = fs::write(&cache_file, cache_json).await;
    }

    Ok(games)
}

#[tauri::command]
pub fn get_game_asset_path(
    app: AppHandle,
    game_id: u32,
    filename: String,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = data_dir
        .join("assets")
        .join(game_id.to_string())
        .join(filename);
    Ok(path.to_string_lossy().to_string())
}

/// Downloads everything needed to play a single game offline: every map's
/// location data and image, the marker sprite sheet sliced by category ID
/// (so the frontend can look up icons by category_id directly).
#[tauri::command]
pub async fn download_game_assets(app: AppHandle, game_id: u32) -> Result<(), String> {
    let game = get_cached_game(&app, game_id).await?;
    let client = build_client()?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let game_dir = data_dir.join("assets").join(game.id.to_string());
    let maps_dir = game_dir.join("maps");
    let markers_dir = game_dir.join("markers");

    // Skip everything if a previous run finished successfully. Without this, every map open
    // re-scrapes the marker page and re-slices icons, which is slow and shows a needless
    // "Downloading map assets" screen each time. The marker is only written on full success,
    // so an interrupted/partial download will correctly re-run.
    let complete_marker = game_dir.join(".assets_complete");
    if fs::try_exists(&complete_marker).await.unwrap_or(false) {
        return Ok(());
    }

    fs::create_dir_all(&maps_dir)
        .await
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&markers_dir)
        .await
        .map_err(|e| e.to_string())?;

    // 1. Per-map location data + map preview image.
    for map in &game.maps {
        let map_data_url = format!("https://mapgenie.io/api/v1/maps/{}/data", map.id);
        if let Ok(resp) = client.get(&map_data_url).send().await {
            if let Ok(bytes) = resp.bytes().await {
                let _ = fs::write(maps_dir.join(format!("{}.json", map.id)), &bytes).await;
            }
        }

        if let Some(img_url) = &map.image {
            let _ = download_file(&client, img_url, maps_dir.join(format!("{}.jpg", map.id))).await;
        }
    }

    // 2. Scrape MARKER_SPRITE_POSITIONS_V3 (category_id → @2x sprite crop coords) from the
    //    first available map page so we can save icons keyed by category_id.
    let first_map = game.maps.first().ok_or("Game has no maps")?;
    let (_, marker_positions_val) = fetch_map_tile_config(&client, &game.slug, &first_map.slug)
        .await
        .unwrap_or_else(|_| (MapConfig { tile_sets: vec![] }, serde_json::Value::Null));

    // 3. Download the @2x sprite sheet (marker_sprite_url ends in @2x.png).
    //    MARKER_SPRITE_POSITIONS_V3 coordinates are for the @2x (retina) sheet, so we must
    //    keep the @2x version — do NOT strip the density suffix. We save it under a distinct
    //    filename (sprite@2x.png) so a stale @1x "sprite.png" from older builds can't block
    //    the download via download_file's exists-check (which would leave V3 crops out of
    //    bounds and silently produce no icons).
    let sprite_url_raw = normalize_url(&game.config.marker_sprite_url, "cdn.mapgenie.io");
    let sprite_path = markers_dir.join("sprite@2x.png");
    download_file(&client, &sprite_url_raw, sprite_path.clone()).await?;

    // 4. Slice the sprite sheet into per-category-ID PNGs using MARKER_SPRITE_POSITIONS_V3.
    //    Fall back to CDN markers.json (icon-name keyed) if V3 data is unavailable.
    if let Some(positions_map) = marker_positions_val.as_object() {
        // V3 path: keys are category_id strings, values have @2x pixel coordinates.
        let positions: HashMap<String, MarkerSpriteEntry> = positions_map
            .iter()
            .filter_map(|(k, v)| serde_json::from_value(v.clone()).ok().map(|e| (k.clone(), e)))
            .collect();

        let slice_markers_dir = markers_dir.clone();
        tokio::task::spawn_blocking(move || {
            slice_marker_sprites(&sprite_path, &slice_markers_dir, &positions)
        })
        .await
        .map_err(|e| e.to_string())??;
    } else {
        // Fallback: V3 positions weren't found on the page. Use the CDN markers.json atlas,
        // whose coordinates are for the @1x sprite — so download the @1x sheet (strip the
        // density suffix) to its own path. These icons end up keyed by icon-name, which the
        // frontend (which looks up by category_id) won't find, but it avoids hard-failing.
        let sprite_1x_url = strip_density_suffix(&sprite_url_raw);
        let sprite_1x_path = markers_dir.join("sprite.png");
        download_file(&client, &sprite_1x_url, sprite_1x_path.clone()).await?;

        let markers_json_url = sibling_markers_json_url(&sprite_url_raw)?;
        let markers_resp = client
            .get(&markers_json_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let markers_bytes = markers_resp.bytes().await.map_err(|e| e.to_string())?;
        let markers_map: HashMap<String, MarkerSpriteEntry> =
            serde_json::from_slice(&markers_bytes).map_err(|e| e.to_string())?;

        fs::write(markers_dir.join("markers.json"), &markers_bytes)
            .await
            .map_err(|e| e.to_string())?;

        let slice_markers_dir = markers_dir.clone();
        tokio::task::spawn_blocking(move || {
            slice_marker_sprites(&sprite_1x_path, &slice_markers_dir, &markers_map)
        })
        .await
        .map_err(|e| e.to_string())??;
    }

    // Mark the download complete so subsequent opens skip all of the above.
    let _ = fs::write(&complete_marker, b"1").await;

    Ok(())
}

/// Returns whether a game's offline assets have already been fully downloaded, so the UI can
/// skip the "Downloading map assets" screen and open the map immediately.
#[tauri::command]
pub async fn game_assets_ready(app: AppHandle, game_id: u32) -> Result<bool, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let marker = data_dir
        .join("assets")
        .join(game_id.to_string())
        .join(".assets_complete");
    Ok(fs::try_exists(&marker).await.unwrap_or(false))
}

/// Downloads all map tiles for a specific map at all zoom levels defined in the
/// tile config. Tiles are stored at their native (Web Mercator) z/x/y coordinates,
/// served directly by the `tile://` protocol handler. Saves a tile_meta.json
/// alongside with extension and zoom range info.
#[tauri::command]
pub async fn download_map_tiles(app: AppHandle, game_id: u32, map_id: u32) -> Result<(), String> {
    let game = get_cached_game(&app, game_id).await?;
    let map = game
        .maps
        .iter()
        .find(|m| m.id == map_id)
        .ok_or_else(|| format!("Map {} not found for game {}", map_id, game_id))?;

    let client = build_client()?;
    let (tile_config, _) = fetch_map_tile_config(&client, &game.slug, &map.slug).await?;
    let tile_set = tile_config
        .tile_sets
        .into_iter()
        .next()
        .ok_or("No tile sets found for this map")?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let map_dir = data_dir
        .join("assets")
        .join(game_id.to_string())
        .join("maps")
        .join(map_id.to_string());
    let tiles_dir = map_dir.join("tiles");

    // tiles_base_url is "https://tiles.mapgenie.io"; CDN path is /games/{pattern}
    let tiles_base = format!(
        "{}/games",
        game.config.tiles_base_url.trim_end_matches('/')
    );

    let semaphore = Arc::new(tokio::sync::Semaphore::new(TILE_DOWNLOAD_CONCURRENCY));
    let mut join_set = tokio::task::JoinSet::new();

    let ext = tile_set.extension.clone();

    for zoom in tile_set.min_zoom..=tile_set.max_zoom {
        let Some(b) = tile_set.bounds.get(&zoom.to_string()) else {
            continue;
        };
        for x in b.x.min..=b.x.max {
            for y in b.y.min..=b.y.max {
                let url = format!(
                    "{}/{}",
                    tiles_base,
                    tile_set
                        .pattern
                        .replace("{z}", &zoom.to_string())
                        .replace("{x}", &x.to_string())
                        .replace("{y}", &y.to_string())
                );
                let out_path = tiles_dir
                    .join(zoom.to_string())
                    .join(x.to_string())
                    .join(format!("{}.{}", y, ext));
                let client = client.clone();
                let sem = semaphore.clone();
                join_set.spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    if let Some(parent) = out_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = download_file(&client, &url, out_path).await;
                });
            }
        }
    }

    while join_set.join_next().await.is_some() {}

    // Save tile metadata so the frontend knows zoom range and file extension, and the
    // tile protocol handler can fetch-on-demand from the CDN URL.
    let meta = TileMeta {
        min_zoom: tile_set.min_zoom,
        max_zoom: tile_set.max_zoom,
        extension: tile_set.extension,
        url_template: format!("{}/{}", tiles_base, tile_set.pattern),
    };
    let meta_json = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    fs::create_dir_all(&map_dir)
        .await
        .map_err(|e| e.to_string())?;
    fs::write(map_dir.join("tile_meta.json"), meta_json)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Downloads tiles for EVERY map in a game so all previews open instantly from disk
/// afterwards. Emits `tile-download-progress` events ({gameId, downloaded, total}) so
/// the UI can show a percentage. Already-downloaded tiles are skipped (download_file
/// short-circuits on existing files) but still counted, so re-running jumps to 100%.
#[tauri::command]
pub async fn download_all_game_tiles(app: AppHandle, game_id: u32) -> Result<(), String> {
    let game = get_cached_game(&app, game_id).await?;
    let client = build_client()?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let tiles_base = format!("{}/games", game.config.tiles_base_url.trim_end_matches('/'));

    // Phase 1: fetch every map's tile config and count the total tiles up front so the
    // progress bar has a stable denominator. A map whose page/config can't be fetched is
    // skipped rather than aborting the whole game.
    struct MapJob {
        map_id: u32,
        tile_set: TileSet,
    }
    let mut jobs: Vec<MapJob> = Vec::new();
    let mut total: u64 = 0;

    for map in &game.maps {
        let Ok((config, _)) = fetch_map_tile_config(&client, &game.slug, &map.slug).await else {
            continue;
        };
        let Some(tile_set) = config.tile_sets.into_iter().next() else {
            continue;
        };
        for zoom in tile_set.min_zoom..=tile_set.max_zoom {
            if let Some(b) = tile_set.bounds.get(&zoom.to_string()) {
                let cols = (b.x.max - b.x.min + 1) as u64;
                let rows = (b.y.max - b.y.min + 1) as u64;
                total += cols * rows;
            }
        }
        jobs.push(MapJob {
            map_id: map.id,
            tile_set,
        });
    }

    let _ = app.emit(
        "tile-download-progress",
        TileProgress {
            game_id,
            downloaded: 0,
            total,
        },
    );

    if total == 0 {
        return Ok(());
    }

    // Phase 2: download everything with a single shared concurrency budget across all maps.
    let downloaded = Arc::new(AtomicU64::new(0));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(TILE_DOWNLOAD_CONCURRENCY));
    let mut join_set = tokio::task::JoinSet::new();
    // Emit ~100 progress updates max, rather than one per tile.
    let step = (total / 100).max(1);

    for job in &jobs {
        let map_tiles_dir = data_dir
            .join("assets")
            .join(game_id.to_string())
            .join("maps")
            .join(job.map_id.to_string())
            .join("tiles");
        let ext = job.tile_set.extension.clone();

        for zoom in job.tile_set.min_zoom..=job.tile_set.max_zoom {
            let Some(b) = job.tile_set.bounds.get(&zoom.to_string()) else {
                continue;
            };
            for x in b.x.min..=b.x.max {
                for y in b.y.min..=b.y.max {
                    let url = format!(
                        "{}/{}",
                        tiles_base,
                        job.tile_set
                            .pattern
                            .replace("{z}", &zoom.to_string())
                            .replace("{x}", &x.to_string())
                            .replace("{y}", &y.to_string())
                    );
                    let out_path = map_tiles_dir
                        .join(zoom.to_string())
                        .join(x.to_string())
                        .join(format!("{}.{}", y, ext));
                    let client = client.clone();
                    let sem = semaphore.clone();
                    let counter = downloaded.clone();
                    let app = app.clone();
                    join_set.spawn(async move {
                        let _permit = sem.acquire().await.unwrap();
                        if let Some(parent) = out_path.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        let _ = download_file(&client, &url, out_path).await;

                        let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
                        if n % step == 0 || n == total {
                            let _ = app.emit(
                                "tile-download-progress",
                                TileProgress {
                                    game_id,
                                    downloaded: n,
                                    total,
                                },
                            );
                        }
                    });
                }
            }
        }
    }

    while join_set.join_next().await.is_some() {}

    // Write tile_meta.json for each map so the map view knows the zoom range/extension
    // and can skip re-downloading.
    for job in &jobs {
        let map_dir = data_dir
            .join("assets")
            .join(game_id.to_string())
            .join("maps")
            .join(job.map_id.to_string());
        let meta = TileMeta {
            min_zoom: job.tile_set.min_zoom,
            max_zoom: job.tile_set.max_zoom,
            extension: job.tile_set.extension.clone(),
            url_template: format!("{}/{}", tiles_base, job.tile_set.pattern),
        };
        if let Ok(meta_json) = serde_json::to_string(&meta) {
            let _ = fs::create_dir_all(&map_dir).await;
            let _ = fs::write(map_dir.join("tile_meta.json"), meta_json).await;
        }
    }

    // Final 100% in case the last emit landed mid-step.
    let _ = app.emit(
        "tile-download-progress",
        TileProgress {
            game_id,
            downloaded: total,
            total,
        },
    );

    Ok(())
}

/// Ensures tile metadata exists for a map WITHOUT downloading any tiles, so the map view
/// can open instantly and let the `tile://` handler stream/cache tiles on demand. Returns
/// cached/on-disk metadata if present, otherwise scrapes the map config and writes it.
#[tauri::command]
pub async fn ensure_tile_meta(
    app: AppHandle,
    game_id: u32,
    map_id: u32,
) -> Result<TileMeta, String> {
    if let Some(meta) = load_tile_meta(&app, game_id, map_id).await {
        return Ok(meta);
    }

    let game = get_cached_game(&app, game_id).await?;
    let map = game
        .maps
        .iter()
        .find(|m| m.id == map_id)
        .ok_or_else(|| format!("Map {} not found for game {}", map_id, game_id))?;

    let client = build_client()?;
    let (tile_config, _) = fetch_map_tile_config(&client, &game.slug, &map.slug).await?;
    let tile_set = tile_config
        .tile_sets
        .into_iter()
        .next()
        .ok_or("No tile sets found for this map")?;

    let tiles_base = format!("{}/games", game.config.tiles_base_url.trim_end_matches('/'));
    let meta = TileMeta {
        min_zoom: tile_set.min_zoom,
        max_zoom: tile_set.max_zoom,
        extension: tile_set.extension,
        url_template: format!("{}/{}", tiles_base, tile_set.pattern),
    };

    let map_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("assets")
        .join(game_id.to_string())
        .join("maps")
        .join(map_id.to_string());
    let _ = fs::create_dir_all(&map_dir).await;
    if let Ok(json) = serde_json::to_string(&meta) {
        let _ = fs::write(map_dir.join("tile_meta.json"), json).await;
    }
    meta_cache()
        .lock()
        .await
        .insert((game_id, map_id), meta.clone());

    Ok(meta)
}

fn meta_cache() -> &'static tokio::sync::Mutex<HashMap<(u32, u32), TileMeta>> {
    TILE_META_CACHE.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

/// Loads tile metadata from the in-memory cache, falling back to tile_meta.json on disk.
async fn load_tile_meta(app: &AppHandle, game_id: u32, map_id: u32) -> Option<TileMeta> {
    {
        let cache = meta_cache().lock().await;
        if let Some(meta) = cache.get(&(game_id, map_id)) {
            return Some(meta.clone());
        }
    }
    let meta_path = app
        .path()
        .app_data_dir()
        .ok()?
        .join("assets")
        .join(game_id.to_string())
        .join("maps")
        .join(map_id.to_string())
        .join("tile_meta.json");
    let content = fs::read_to_string(&meta_path).await.ok()?;
    let meta: TileMeta = serde_json::from_str(&content).ok()?;
    // Treat a missing url_template as a cache miss so ensure_tile_meta re-scrapes fresh data.
    // Old tile_meta.json files written before this field was introduced have url_template = "".
    if meta.url_template.is_empty() {
        return None;
    }
    meta_cache()
        .lock()
        .await
        .insert((game_id, map_id), meta.clone());
    Some(meta)
}

/// Serves a tile for the `tile://` protocol: returns it from disk if cached, otherwise
/// fetches it from the CDN on the fly, caches it to disk, and returns the bytes. This is
/// what lets the map open instantly and sharpen as tiles stream in. Every response carries
/// `Access-Control-Allow-Origin` so cross-origin (localhost → tile.localhost) requests pass.
pub async fn serve_tile_request(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let cors_error = |status: StatusCode| {
        Response::builder()
            .status(status)
            .header("Access-Control-Allow-Origin", "*")
            .body(Vec::new())
            .unwrap()
    };

    // Path: /{game_id}/{map_id}/{z}/{x}/{y}
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.len() != 5 {
        return cors_error(StatusCode::BAD_REQUEST);
    }
    let (Ok(game_id), Ok(map_id)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) else {
        return cors_error(StatusCode::BAD_REQUEST);
    };
    let (z, x) = (parts[2], parts[3]);
    let y = parts[4].split('.').next().unwrap_or(parts[4]);

    let Some(meta) = load_tile_meta(app, game_id, map_id).await else {
        return cors_error(StatusCode::NOT_FOUND);
    };
    let content_type = if meta.extension == "jpg" || meta.extension == "jpeg" {
        "image/jpeg"
    } else {
        "image/png"
    };

    let Ok(data_dir) = app.path().app_data_dir() else {
        return cors_error(StatusCode::INTERNAL_SERVER_ERROR);
    };
    let tile_path = data_dir
        .join("assets")
        .join(game_id.to_string())
        .join("maps")
        .join(map_id.to_string())
        .join("tiles")
        .join(z)
        .join(x)
        .join(format!("{}.{}", y, meta.extension));

    let ok_response = |bytes: Vec<u8>| {
        Response::builder()
            .header("Content-Type", content_type)
            .header("Access-Control-Allow-Origin", "*")
            .body(bytes)
            .unwrap()
    };

    // 1. Cache hit on disk.
    if let Ok(bytes) = fs::read(&tile_path).await {
        return ok_response(bytes);
    }

    // 2. Cache miss → fetch from CDN, then persist for offline use.
    if meta.url_template.is_empty() {
        return cors_error(StatusCode::NOT_FOUND);
    }
    let url = meta
        .url_template
        .replace("{z}", z)
        .replace("{x}", x)
        .replace("{y}", y);
    let Ok(resp) = tile_client().get(&url).send().await else {
        return cors_error(StatusCode::BAD_GATEWAY);
    };
    if !resp.status().is_success() {
        // Out-of-bounds tiles legitimately 404 on sparse maps.
        return cors_error(StatusCode::NOT_FOUND);
    }
    let Ok(bytes) = resp.bytes().await else {
        return cors_error(StatusCode::BAD_GATEWAY);
    };
    let bytes = bytes.to_vec();

    if let Some(parent) = tile_path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    let _ = fs::write(&tile_path, &bytes).await;

    ok_response(bytes)
}

// --- Private helpers ---

/// Crops each named region out of the sprite sheet and saves it as its own PNG.
fn slice_marker_sprites(
    sprite_path: &PathBuf,
    out_dir: &PathBuf,
    markers: &HashMap<String, MarkerSpriteEntry>,
) -> Result<(), String> {
    let sheet = image::open(sprite_path).map_err(|e| e.to_string())?;
    let (sheet_w, sheet_h) = (sheet.width(), sheet.height());

    for (name, entry) in markers {
        if entry.width == 0 || entry.height == 0 {
            continue;
        }
        if entry.x + entry.width > sheet_w || entry.y + entry.height > sheet_h {
            continue;
        }

        let cropped = sheet.crop_imm(entry.x, entry.y, entry.width, entry.height);
        let out_path = out_dir.join(format!("{}.png", name));
        cropped.save(&out_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn download_file(client: &Client, url: &str, path: PathBuf) -> Result<(), String> {
    if fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(());
    }
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {}", resp.status(), url));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&path, bytes).await.map_err(|e| e.to_string())?;
    Ok(())
}

fn sibling_markers_json_url(sprite_url: &str) -> Result<String, String> {
    // Strip query string first
    let url_no_query = sprite_url.split('?').next().unwrap_or(sprite_url);
    // Strip @2x/@1x density suffix if present
    let url_clean = strip_density_suffix(url_no_query);
    let last_slash = url_clean
        .rfind('/')
        .ok_or_else(|| format!("Unexpected marker sprite URL format: {}", sprite_url))?;
    Ok(format!("{}/markers.json", &url_clean[..last_slash]))
}

fn normalize_url(url: &str, default_host: &str) -> String {
    if url.starts_with("//") {
        format!("https:{}", url)
    } else if url.starts_with('/') {
        format!("https://{}{}", default_host, url)
    } else {
        url.to_string()
    }
}

fn strip_density_suffix(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(dot_pos) = url[at_pos..].find('.') {
            let after_at = &url[at_pos + 1..at_pos + dot_pos];
            if after_at.ends_with('x')
                && after_at[..after_at.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit())
            {
                return format!("{}{}", &url[..at_pos], &url[at_pos + dot_pos..]);
            }
        }
    }
    url.to_string()
}
