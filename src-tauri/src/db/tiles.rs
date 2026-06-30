//! MBTiles-style tile blob storage: one indexed SQLite table instead of one
//! OS file per tile. A fully-downloaded game used to write hundreds of
//! thousands of tiny files (huge filesystem/inode overhead, slow to
//! enumerate/delete, especially bad on mobile); this collapses them into a
//! single file with an indexed primary key lookup.

use rusqlite::{params, Connection, OptionalExtension};

const CURRENT_VERSION: u32 = 1;

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version >= CURRENT_VERSION {
        return Ok(());
    }
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS tiles (
           game_id  INTEGER NOT NULL,
           map_id   INTEGER NOT NULL,
           z        INTEGER NOT NULL,
           x        INTEGER NOT NULL,
           y        INTEGER NOT NULL,
           ext      TEXT    NOT NULL,
           data     BLOB    NOT NULL,
           PRIMARY KEY (game_id, map_id, z, x, y)
         );
         PRAGMA user_version = 1;
         COMMIT;",
    )
}

/// Fetches one cached tile's bytes + extension, or `None` on a cache miss.
pub fn get_tile(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
    z: u32,
    x: u32,
    y: u32,
) -> Result<Option<(Vec<u8>, String)>, rusqlite::Error> {
    conn.query_row(
        "SELECT data, ext FROM tiles WHERE game_id=?1 AND map_id=?2 AND z=?3 AND x=?4 AND y=?5",
        params![game_id, map_id, z, x, y],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// Stores one tile's bytes. `INSERT OR REPLACE` so a re-download (e.g. after a
/// stale/corrupt fetch) simply overwrites rather than erroring on the existing
/// primary key.
pub fn put_tile(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
    z: u32,
    x: u32,
    y: u32,
    ext: &str,
    data: &[u8],
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO tiles (game_id, map_id, z, x, y, ext, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![game_id, map_id, z, x, y, ext, data],
    )?;
    Ok(())
}

/// Whether a tile is already cached — used by the bulk downloaders to skip
/// re-fetching, mirroring the old `download_file`'s exists-check.
pub fn has_tile(
    conn: &Connection,
    game_id: u32,
    map_id: u32,
    z: u32,
    x: u32,
    y: u32,
) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT 1 FROM tiles WHERE game_id=?1 AND map_id=?2 AND z=?3 AND x=?4 AND y=?5",
        params![game_id, map_id, z, x, y],
        |_| Ok(()),
    )
    .optional()
    .map(|o| o.is_some())
}

/// Deletes every cached tile for a single map (used by a future "clear tile
/// cache" action). Tiles are fully disposable — re-fetchable from the CDN.
#[allow(dead_code)]
pub fn clear_map_tiles(conn: &Connection, game_id: u32, map_id: u32) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM tiles WHERE game_id=?1 AND map_id=?2",
        params![game_id, map_id],
    )?;
    Ok(())
}
