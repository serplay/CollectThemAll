//! Queries against userdata.sqlite: found-marker state and custom markers.
//! Kept as plain functions over `&Connection` so the `#[tauri::command]`
//! wrappers in `commands::userdata` just lock the Mutex and delegate here.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ── Found markers ────────────────────────────────────────────────────────

pub fn get_found_ids(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
) -> Result<Vec<i64>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT location_id FROM found_markers WHERE game_id=?1 AND map_id=?2",
    )?;
    let rows = stmt.query_map(params![game_id, map_id], |r| r.get(0))?;
    rows.collect()
}

pub fn set_found(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
    location_id: i64,
    found: bool,
) -> Result<(), rusqlite::Error> {
    if found {
        conn.execute(
            "INSERT OR IGNORE INTO found_markers (game_id, map_id, location_id, found_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![game_id, map_id, location_id, now()],
        )?;
    } else {
        conn.execute(
            "DELETE FROM found_markers WHERE game_id=?1 AND map_id=?2 AND location_id=?3",
            params![game_id, map_id, location_id],
        )?;
    }
    Ok(())
}

/// Replaces the entire found-set for a game+map in one transaction — used both
/// by "clear all" (empty ids) and to apply a full set pushed from the other
/// window via the `cta:found-changed` event.
pub fn set_found_bulk(
    conn: &mut Connection,
    game_id: u32,
    map_id: u32,
    ids: &[i64],
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM found_markers WHERE game_id=?1 AND map_id=?2",
        params![game_id, map_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO found_markers (game_id, map_id, location_id, found_at) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let t = now();
        for id in ids {
            stmt.execute(params![game_id, map_id, id, t])?;
        }
    }
    tx.commit()
}

// ── Custom markers ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomMarker {
    pub id: i64,
    pub game_id: u32,
    pub map_id: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub title: String,
    pub description: String,
    pub color: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_marker(r: &rusqlite::Row) -> Result<CustomMarker, rusqlite::Error> {
    Ok(CustomMarker {
        id: r.get(0)?,
        game_id: r.get(1)?,
        map_id: r.get(2)?,
        latitude: r.get(3)?,
        longitude: r.get(4)?,
        title: r.get(5)?,
        description: r.get(6)?,
        color: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

const SELECT_COLS: &str =
    "id, game_id, map_id, latitude, longitude, title, description, color, created_at, updated_at";

pub fn list_custom_markers(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
) -> Result<Vec<CustomMarker>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM custom_markers WHERE game_id=?1 AND map_id=?2 ORDER BY id"
    ))?;
    let rows = stmt.query_map(params![game_id, map_id], row_to_marker)?;
    rows.collect()
}

pub fn add_custom_marker(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
    latitude: f64,
    longitude: f64,
    title: &str,
    description: &str,
) -> Result<CustomMarker, rusqlite::Error> {
    let t = now();
    conn.execute(
        "INSERT INTO custom_markers
           (game_id, map_id, latitude, longitude, title, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![game_id, map_id, latitude, longitude, title, description, t],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM custom_markers WHERE id=?1"),
        params![id],
        row_to_marker,
    )
}

pub fn update_custom_marker(
    conn: &Connection,
    id: i64,
    title: &str,
    description: &str,
) -> Result<Option<CustomMarker>, rusqlite::Error> {
    conn.execute(
        "UPDATE custom_markers SET title=?1, description=?2, updated_at=?3 WHERE id=?4",
        params![title, description, now(), id],
    )?;
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM custom_markers WHERE id=?1"),
        params![id],
        row_to_marker,
    )
    .optional()
}

pub fn delete_custom_marker(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM custom_markers WHERE id=?1", params![id])?;
    Ok(())
}

// ── Migration bookkeeping (one-off localStorage → SQLite import) ────────────

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row("SELECT value FROM meta WHERE key=?1", params![key], |r| r.get(0))
        .optional()
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}
