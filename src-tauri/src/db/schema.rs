//! SQLite schema definition and user_version-based migration runner.
//!
//! Each migration is a list of SQL statements applied in one transaction.
//! user_version acts as the schema generation counter — we only run statements
//! for versions strictly above the current one, so applying twice is a no-op.
//! Never edit an old migration; always add a new version instead.

use rusqlite::Connection;

const CURRENT_VERSION: u32 = 3;

/// Bring the schema up to date. Must be called once right after the connection
/// is opened, before the connection is handed out to any command.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version >= CURRENT_VERSION {
        return Ok(());
    }

    // Each branch runs every migration from (version+1) up to CURRENT_VERSION in a
    // single transaction so partial failure leaves the schema unchanged.
    conn.execute_batch("BEGIN;")?;

    if version < 1 {
        conn.execute_batch(V1)?;
    }
    if version < 2 {
        conn.execute_batch(V2)?;
    }
    if version < 3 {
        conn.execute_batch(V3)?;
    }

    conn.execute_batch(&format!("PRAGMA user_version = {CURRENT_VERSION}; COMMIT;"))?;
    Ok(())
}

// ── v1: found markers (replaces localStorage found:{game}:{map} keys) ──────
const V1: &str = "
CREATE TABLE IF NOT EXISTS found_markers (
  game_id     INTEGER NOT NULL,
  map_id      INTEGER NOT NULL,
  location_id INTEGER NOT NULL,
  found_at    INTEGER NOT NULL,
  PRIMARY KEY (game_id, map_id, location_id)
);
";

// ── v2: user-created custom markers ─────────────────────────────────────────
const V2: &str = "
CREATE TABLE IF NOT EXISTS custom_markers (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  game_id     INTEGER NOT NULL,
  map_id      INTEGER NOT NULL,
  latitude    REAL    NOT NULL,
  longitude   REAL    NOT NULL,
  title       TEXT    NOT NULL DEFAULT '',
  description TEXT    NOT NULL DEFAULT '',
  color       TEXT,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_custom_markers_map
  ON custom_markers (game_id, map_id);
";

// ── v3: migration-state table (tracks one-off localStorage imports) ──────────
const V3: &str = "
CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";
