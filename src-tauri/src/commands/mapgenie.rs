use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tokio::fs;

// --- Data Structures (unchanged) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: u32,
    pub title: String,
    pub status: String,
    pub image: Option<String>,
    pub logo: Option<String>,
    pub config: GameConfig,
    pub maps: Vec<Map>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub cdn_url: String,
    pub tiles_base_url: String,
    pub presets_enabled: bool,
    pub marker_sprite_url: String,
    pub compass_enabled: bool,
    pub heatmaps_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Map {
    pub id: u32,
    pub game_id: u32,
    pub title: String,
    pub slug: String,
    pub image: Option<String>,
    pub order: i32,
    pub enabled: bool,
    pub available: bool,
    pub work_in_progress: bool,
    pub initial_zoom: i32,
    pub initial_latitude: f64,
    pub initial_longitude: f64,
    pub locations_count: u32,
    pub map_style: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    timestamp: u64,
    games: Vec<Game>,
}

// --- Commands ---

#[tauri::command]
pub async fn fetch_and_cache_games(app: AppHandle) -> Result<Vec<Game>, String> {
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let cache_file = cache_dir.join("mapgenie_games_cache.json");

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let twelve_hours = 12 * 60 * 60;

    // 1. Check existing cache
    if let Ok(cache_content) = fs::read_to_string(&cache_file).await {
        if let Ok(cache_data) = serde_json::from_str::<CacheData>(&cache_content) {
            if now - cache_data.timestamp < twelve_hours {
                return Ok(cache_data.games);
            }
        }
    }

    // 2. Fetch fresh data
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://mapgenie.io/api/v1/games")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let games: Vec<Game> = response.json().await.map_err(|e| e.to_string())?;

    // 3. Prepare assets directory — keyed by game id, not sanitized title
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

    // 4. Update cache
    let cache_data = CacheData { timestamp: now, games: games.clone() };
    if let Ok(cache_json) = serde_json::to_string(&cache_data) {
        let _ = fs::create_dir_all(&cache_dir).await;
        let _ = fs::write(&cache_file, cache_json).await;
    }

    Ok(games)
}

/// Resolves the absolute filesystem path for a cached game asset so the
/// frontend can turn it into a webview-loadable URL via `convertFileSrc`.
#[tauri::command]
pub fn get_game_asset_path(app: AppHandle, game_id: u32, filename: String) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = data_dir.join("assets").join(game_id.to_string()).join(filename);
    Ok(path.to_string_lossy().to_string())
}

async fn download_file(client: &Client, url: &str, path: PathBuf) -> Result<(), String> {
    if fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(());
    }
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&path, bytes).await.map_err(|e| e.to_string())?;
    Ok(())
}