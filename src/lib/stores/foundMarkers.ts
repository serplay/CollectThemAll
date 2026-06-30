/**
 * Persistence layer for tracking which map locations the player has found.
 *
 * Data lives in `userdata.sqlite` on the Rust side (see `src-tauri/src/db/userdata.rs`)
 * rather than localStorage — this is the only door into that table, going through the
 * `#[tauri::command]`s registered in `lib.rs`. Doing it this way keeps the trust boundary
 * intact (the frontend never runs SQL directly) and gives us a real file we can back up,
 * export, or eventually sync.
 */

import { invoke } from '@tauri-apps/api/core';

/** Load the set of found location IDs for a specific game map. */
export async function loadFoundIds(gameId: number, mapId: number): Promise<Set<number>> {
  const ids = await invoke<number[]>('get_found_ids', { gameId, mapId });
  return new Set(ids);
}

/** Replace the full set of found location IDs (used to apply a change pushed from another
 *  window, since separate webviews don't share an in-memory cache). */
export async function setFoundIds(gameId: number, mapId: number, ids: Set<number>): Promise<void> {
  await invoke('set_found_bulk', { gameId, mapId, ids: [...ids] });
}

/** Mark a single location as found. Returns the updated set. */
export async function markFound(
  gameId: number,
  mapId: number,
  locationId: number,
): Promise<Set<number>> {
  await invoke('set_found', { gameId, mapId, locationId, found: true });
  return loadFoundIds(gameId, mapId);
}

/** Unmark a single location (mark as not found). Returns the updated set. */
export async function unmarkFound(
  gameId: number,
  mapId: number,
  locationId: number,
): Promise<Set<number>> {
  await invoke('set_found', { gameId, mapId, locationId, found: false });
  return loadFoundIds(gameId, mapId);
}

/** Toggle a location's found state. Returns [updatedSet, isNowFound]. */
export async function toggleFound(
  gameId: number,
  mapId: number,
  locationId: number,
): Promise<[Set<number>, boolean]> {
  const ids = await loadFoundIds(gameId, mapId);
  const wasFound = ids.has(locationId);
  await invoke('set_found', { gameId, mapId, locationId, found: !wasFound });
  if (wasFound) {
    ids.delete(locationId);
  } else {
    ids.add(locationId);
  }
  return [ids, !wasFound];
}

/** Clear all found markers for a specific game map. */
export async function clearFound(gameId: number, mapId: number): Promise<void> {
  await invoke('clear_found', { gameId, mapId });
}

// ── One-time localStorage → SQLite import ──────────────────────────────────

const LEGACY_PREFIX = 'found:';
const MIGRATION_FLAG = 'cta:foundMigratedToSqlite';

/**
 * Scrapes any leftover `found:{gameId}:{mapId}` keys out of localStorage (the
 * old persistence format) and imports them into SQLite once. Safe to call on
 * every app startup — both the localStorage flag here and a matching `meta`
 * row on the Rust side make repeat calls a no-op. We intentionally leave the
 * old localStorage keys in place for one release as a safety net rather than
 * deleting them immediately.
 */
export async function migrateLegacyFoundMarkers(): Promise<void> {
  try {
    if (localStorage.getItem(MIGRATION_FLAG)) return;

    const entries: { gameId: number; mapId: number; ids: number[] }[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (!key || !key.startsWith(LEGACY_PREFIX)) continue;
      const [, gameIdStr, mapIdStr] = key.split(':');
      const gameId = Number(gameIdStr);
      const mapId = Number(mapIdStr);
      if (!Number.isFinite(gameId) || !Number.isFinite(mapId)) continue;
      try {
        const raw = localStorage.getItem(key);
        const parsed = raw ? JSON.parse(raw) : [];
        if (Array.isArray(parsed) && parsed.length > 0) {
          entries.push({ gameId, mapId, ids: parsed });
        }
      } catch {
        // Corrupted entry — skip it rather than failing the whole migration.
      }
    }

    if (entries.length > 0) {
      await invoke('import_found_from_storage', { entries });
    }
    localStorage.setItem(MIGRATION_FLAG, '1');
  } catch {
    // Not under Tauri, or storage unavailable — non-fatal, nothing to migrate yet.
  }
}
