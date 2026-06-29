// API calls and types about map *tiles* (the little image squares a map is made of).
//
// Cybersecurity note: the heavy tile downloading happens in the Rust backend, not
// here. The frontend only kicks it off and listens for progress events. Doing the
// risky network work on the trusted side keeps untrusted JavaScript out of it.

import { invoke } from '@tauri-apps/api/core';
import { readTextFile } from '@tauri-apps/plugin-fs';

/** Shape of the progress events emitted while tiles download. */
export interface TileProgress {
  gameId: number;
  downloaded: number;
  total: number;
}

/** Small bit of metadata describing a map's tiles. */
export interface TileMeta {
  min_zoom: number;
  max_zoom: number;
  extension: string;
  url_template?: string;
}

/** Download every tile for a single map. */
export async function downloadMapTiles(gameId: number, mapId: number): Promise<void> {
  return invoke('download_map_tiles', { gameId, mapId });
}

/** Downloads tiles for every map in the game. Progress is reported via the
 *  `tile-download-progress` event ({ gameId, downloaded, total }). */
export async function downloadAllGameTiles(gameId: number): Promise<void> {
  return invoke('download_all_game_tiles', { gameId });
}

/** Read a map's tile_meta.json from disk. */
export async function getTileMeta(gameId: number, mapId: number): Promise<TileMeta> {
  const path = await invoke<string>('get_game_asset_path', {
    gameId,
    filename: `maps/${mapId}/tile_meta.json`,
  });
  const raw = await readTextFile(path);
  return JSON.parse(raw) as TileMeta;
}

/** Ensures tile metadata exists (scrapes the map config once if needed) WITHOUT
 *  downloading tiles, so the map can open immediately and stream tiles on demand. */
export async function ensureTileMeta(gameId: number, mapId: number): Promise<TileMeta> {
  return invoke<TileMeta>('ensure_tile_meta', { gameId, mapId });
}
