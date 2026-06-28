//! Fetching a MapGenie map page and pulling the config we need out of its HTML.
//!
//! Cybersecurity note: this is the step where we actually reach out to the
//! network and read back a page we do not control. We download the HTML, then
//! hand it to our careful parsers. Everything that comes back is treated as
//! untrusted until it successfully matches one of our typed structs.

use reqwest::Client;

use super::parsing::extract_named_var_json;
use super::tile_config::{MapConfig, MapDataHtml};

/// Downloads a map's web page and extracts both the tile config and the raw
/// `mapData` JSON (so callers can also dig out MARKER_SPRITE_POSITIONS_V3).
pub async fn fetch_map_tile_config(
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
