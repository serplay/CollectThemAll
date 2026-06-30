//! Database plumbing: two separate SQLite files, opened once at startup and
//! held in Tauri's managed state.
//!
//! `userdata.sqlite` (app_data_dir) holds found-marker state and custom
//! markers — small and precious, worth backing up. `tiles.sqlite`
//! (app_cache_dir) holds the downloaded map tile blobs — large and fully
//! disposable (re-fetchable from the CDN), kept in a separate file so
//! clearing the tile cache can never touch user data.
//!
//! rusqlite's `Connection` is not `Send`-free-for-await — we never hold the
//! lock across an `.await`. Heavy work (bulk tile writes) runs inside
//! `spawn_blocking` and only touches the connection there.

use rusqlite::Connection;
use std::path::Path;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

pub mod schema;
pub mod tiles;
pub mod userdata;

pub struct UserDataDb(pub Mutex<Connection>);
pub struct TilesDb(pub Mutex<Connection>);

fn open_with_wal(path: &Path) -> Result<Connection, rusqlite::Error> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

/// Same as `open_with_wal` but relaxes fsync-per-write (`synchronous=NORMAL`).
/// Tiles are a disposable, re-fetchable cache — losing the last few writes on
/// a hard crash just means re-fetching those tiles, so we trade a sliver of
/// durability for much less fsync overhead during bulk tile downloads.
fn open_tiles_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = open_with_wal(path)?;
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
    Ok(conn)
}

/// Opens (creating if needed) both database files and runs their migrations.
/// Call once during Tauri's `.setup()`, before any command can be invoked.
pub fn init(app: &AppHandle) -> Result<(UserDataDb, TilesDb), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;

    let user_conn = open_with_wal(&data_dir.join("userdata.sqlite")).map_err(|e| e.to_string())?;
    schema::run_migrations(&user_conn).map_err(|e| e.to_string())?;

    let tiles_conn = open_tiles_db(&cache_dir.join("tiles.sqlite")).map_err(|e| e.to_string())?;
    tiles::run_migrations(&tiles_conn).map_err(|e| e.to_string())?;

    Ok((UserDataDb(Mutex::new(user_conn)), TilesDb(Mutex::new(tiles_conn))))
}
