//! Data models (the "shapes" of our data).
//!
//! Cybersecurity lab note: one of the first things we learned is that you should
//! always *know your data* before you trust it. These structs describe exactly
//! what we expect to receive from the MapGenie servers and what we store on disk.
//! `serde` (the (de)serialize library) uses these definitions to turn untrusted
//! JSON text into typed Rust values — if the JSON does not match the shape below,
//! parsing fails loudly instead of letting bad data sneak in.

use serde::{Deserialize, Serialize};

/// One game in the library (for example "Elden Ring").
///
/// `Serialize` + `Deserialize` mean this can be both written to / read from JSON.
/// `Clone` lets us make copies (we cache a copy and also return one to the frontend).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: u32,
    pub title: String,
    pub slug: String,
    pub status: String,
    // `Option<String>` == "this might be missing". The image/logo URLs are not
    // always present, so we model that honestly instead of assuming they exist.
    pub image: Option<String>,
    pub logo: Option<String>,
    pub config: GameConfig,
    pub maps: Vec<Map>,
}

/// Per-game settings that tell us where the tiles and marker icons live.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub cdn_url: String,
    pub tiles_base_url: String,
    pub presets_enabled: bool,
    pub marker_sprite_url: String,
    pub compass_enabled: bool,
    pub heatmaps_enabled: bool,
}

/// A single map that belongs to a game (a game can have several maps).
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

/// What we actually save to the games-list cache file on disk.
///
/// We store a `timestamp` next to the data so that later we can check "is this
/// cache too old?" — a small example of not trusting stale information forever.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheData {
    pub timestamp: u64,
    pub games: Vec<Game>,
}

/// One rectangle inside the big marker "sprite sheet" image.
///
/// A sprite sheet packs many tiny icons into one picture; these numbers tell us
/// which pixels to cut out for a given icon. `Copy` is fine here because it is
/// just six small numbers (cheap to duplicate).
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct MarkerSpriteEntry {
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
    // The JSON calls this "pixelRatio" (camelCase), but Rust style is snake_case,
    // so we rename it on the way in. Keeping names tidy avoids silly bugs.
    #[serde(rename = "pixelRatio")]
    pub pixel_ratio: u32,
}

/// Small bit of metadata we keep about a map's tiles (saved as tile_meta.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMeta {
    pub min_zoom: u32,
    pub max_zoom: u32,
    pub extension: String,
    /// Full CDN URL with `{z}`/`{x}`/`{y}` placeholders, used by the tile protocol
    /// handler to fetch-on-demand when a tile isn't cached on disk yet.
    #[serde(default)]
    pub url_template: String,
}

/// Progress update that we send to the user interface while downloading tiles,
/// so the loading bar can show a friendly percentage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileProgress {
    pub game_id: u32,
    pub downloaded: u64,
    pub total: u64,
}
