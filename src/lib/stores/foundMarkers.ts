/**
 * Persistence layer for tracking which map locations the player has found.
 *
 * Data is stored in localStorage keyed by `found:{gameId}:{mapId}` so each
 * game+map combo gets its own independent set. The store is intentionally
 * frontend-only — no Tauri backend commands needed.
 */

function storageKey(gameId: number, mapId: number): string {
  return `found:${gameId}:${mapId}`;
}

/** Load the set of found location IDs for a specific game map. */
export function loadFoundIds(gameId: number, mapId: number): Set<number> {
  try {
    const raw = localStorage.getItem(storageKey(gameId, mapId));
    if (!raw) return new Set();
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return new Set(parsed);
    return new Set();
  } catch {
    return new Set();
  }
}

/** Persist the full set of found location IDs. */
function saveFoundIds(gameId: number, mapId: number, ids: Set<number>): void {
  localStorage.setItem(storageKey(gameId, mapId), JSON.stringify([...ids]));
}

/** Replace the full set of found location IDs (used to apply a change pushed from another
 *  window, since WebView windows don't share their in-memory localStorage cache live). */
export function setFoundIds(gameId: number, mapId: number, ids: Set<number>): void {
  saveFoundIds(gameId, mapId, ids);
}

/** Mark a single location as found. Returns the updated set. */
export function markFound(gameId: number, mapId: number, locationId: number): Set<number> {
  const ids = loadFoundIds(gameId, mapId);
  ids.add(locationId);
  saveFoundIds(gameId, mapId, ids);
  return ids;
}

/** Unmark a single location (mark as not found). Returns the updated set. */
export function unmarkFound(gameId: number, mapId: number, locationId: number): Set<number> {
  const ids = loadFoundIds(gameId, mapId);
  ids.delete(locationId);
  saveFoundIds(gameId, mapId, ids);
  return ids;
}

/** Toggle a location's found state. Returns [updatedSet, isNowFound]. */
export function toggleFound(
  gameId: number,
  mapId: number,
  locationId: number
): [Set<number>, boolean] {
  const ids = loadFoundIds(gameId, mapId);
  const wasFound = ids.has(locationId);
  if (wasFound) {
    ids.delete(locationId);
  } else {
    ids.add(locationId);
  }
  saveFoundIds(gameId, mapId, ids);
  return [ids, !wasFound];
}

/** Clear all found markers for a specific game map. */
export function clearFound(gameId: number, mapId: number): void {
  localStorage.removeItem(storageKey(gameId, mapId));
}
