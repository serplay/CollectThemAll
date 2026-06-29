//! Reading things back out of our on-disk / in-memory caches.
//!
//! Cybersecurity note: caching is a classic "availability vs. freshness"
//! trade-off. Keeping a copy on disk means the app still works offline (good for
//! availability), but a cache can also go stale, so callers must decide when the
//! cached value is still good enough to trust. We keep two layers here: a fast
//! in-memory map and the slower on-disk JSON files.

use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};
use tokio::fs;

use super::models::{CacheData, Game, TileMeta};

/// In-memory cache of per-map tile metadata (zoom range, extension, CDN URL template)
/// so the protocol handler doesn't re-read/parse tile_meta.json on every tile request.
static TILE_META_CACHE: OnceLock<tokio::sync::Mutex<HashMap<(u32, u32), TileMeta>>> =
    OnceLock::new();

/// Reads one game out of the games-list cache file we saved earlier.
///
/// Note how we surface a friendly error ("fetch the games list first") instead of
/// leaking the raw I/O error to the user — small example of not over-sharing
/// internal details in error messages.
pub async fn get_cached_game(app: &AppHandle, game_id: u32) -> Result<Game, String> {
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

/// Accessor for the lazily-created in-memory tile-meta cache.
pub fn meta_cache() -> &'static tokio::sync::Mutex<HashMap<(u32, u32), TileMeta>> {
    TILE_META_CACHE.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

/// Loads tile metadata from the in-memory cache, falling back to tile_meta.json on disk.
pub async fn load_tile_meta(app: &AppHandle, game_id: u32, map_id: u32) -> Option<TileMeta> {
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
