//! Tauri commands for found-marker state and custom markers, backed by
//! userdata.sqlite (see `db::userdata`). These replace the old
//! localStorage-based persistence in `src/lib/stores/foundMarkers.ts`.

use tauri::State;

use crate::db::userdata::{self, CustomMarker};
use crate::db::UserDataDb;

#[tauri::command]
pub async fn get_found_ids(
    db: State<'_, UserDataDb>,
    game_id: u32,
    map_id: u32,
) -> Result<Vec<i64>, String> {
    let conn = db.0.lock().await;
    userdata::get_found_ids(&conn, game_id, map_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_found(
    db: State<'_, UserDataDb>,
    game_id: u32,
    map_id: u32,
    location_id: i64,
    found: bool,
) -> Result<(), String> {
    let conn = db.0.lock().await;
    userdata::set_found(&conn, game_id, map_id, location_id, found).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_found_bulk(
    db: State<'_, UserDataDb>,
    game_id: u32,
    map_id: u32,
    ids: Vec<i64>,
) -> Result<(), String> {
    let mut conn = db.0.lock().await;
    userdata::set_found_bulk(&mut conn, game_id, map_id, &ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_found(db: State<'_, UserDataDb>, game_id: u32, map_id: u32) -> Result<(), String> {
    let mut conn = db.0.lock().await;
    userdata::set_found_bulk(&mut conn, game_id, map_id, &[]).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_custom_markers(
    db: State<'_, UserDataDb>,
    game_id: u32,
    map_id: u32,
) -> Result<Vec<CustomMarker>, String> {
    let conn = db.0.lock().await;
    userdata::list_custom_markers(&conn, game_id, map_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_custom_marker(
    db: State<'_, UserDataDb>,
    game_id: u32,
    map_id: u32,
    latitude: f64,
    longitude: f64,
    title: String,
    description: String,
) -> Result<CustomMarker, String> {
    let conn = db.0.lock().await;
    userdata::add_custom_marker(&conn, game_id, map_id, latitude, longitude, &title, &description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_custom_marker(
    db: State<'_, UserDataDb>,
    id: i64,
    title: String,
    description: String,
) -> Result<Option<CustomMarker>, String> {
    let conn = db.0.lock().await;
    userdata::update_custom_marker(&conn, id, &title, &description).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_custom_marker(db: State<'_, UserDataDb>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().await;
    userdata::delete_custom_marker(&conn, id).map_err(|e| e.to_string())
}

/// One-shot import of localStorage's `found:{gameId}:{mapId}` keys into SQLite.
/// The frontend calls this once on startup and passes the entries it scraped
/// out of localStorage, since Rust has no access to the webview's localStorage.
/// Guarded by a `meta` row so a repeat call (e.g. the frontend's own guard
/// failing to persist) is a safe no-op rather than re-clobbering newer data.
#[tauri::command]
pub async fn import_found_from_storage(
    db: State<'_, UserDataDb>,
    entries: Vec<FoundImportEntry>,
) -> Result<(), String> {
    const MIGRATION_KEY: &str = "found_migrated_from_local_storage_v1";
    let mut conn = db.0.lock().await;
    if userdata::get_meta(&conn, MIGRATION_KEY)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(());
    }
    for entry in entries {
        userdata::set_found_bulk(&mut conn, entry.game_id, entry.map_id, &entry.ids)
            .map_err(|e| e.to_string())?;
    }
    userdata::set_meta(&conn, MIGRATION_KEY, "1").map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct FoundImportEntry {
    pub game_id: u32,
    pub map_id: u32,
    pub ids: Vec<i64>,
}
