import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { exists, readTextFile } from '@tauri-apps/plugin-fs';
import type { Game } from '../types/mapgenie';

export async function fetchGames(): Promise<Game[]> {
  return invoke<Game[]>('fetch_and_cache_games');
}

export async function downloadGameAssets(gameId: number): Promise<void> {
  return invoke('download_game_assets', { gameId });
}

/** Whether a game's offline assets are already fully downloaded (lets the UI skip the
 *  "Downloading map assets" screen on repeat opens). */
export async function gameAssetsReady(gameId: number): Promise<boolean> {
  return invoke<boolean>('game_assets_ready', { gameId });
}

export async function getLocalImageAssetUrl(gameId: number, filename: string): Promise<string> {
  const path = await invoke<string>('get_game_asset_path', { gameId, filename });
  if (!(await exists(path))) throw new Error('Asset not cached yet');
  return convertFileSrc(path);
}

export async function getMapData(gameId: number, mapId: number): Promise<any> {
  const path = await invoke<string>('get_game_asset_path', {
    gameId,
    filename: `maps/${mapId}.json`,
  });
  const raw = await readTextFile(path);
  return JSON.parse(raw);
}

export async function getMarkerIconUrl(gameId: number, categoryId: number): Promise<string | null> {
  try {
    return await getLocalImageAssetUrl(gameId, `markers/${categoryId}.png`);
  } catch {
    return null; // icon missing — caller should fall back to a generic pin
  }
}

export async function downloadMapTiles(gameId: number, mapId: number): Promise<void> {
  return invoke('download_map_tiles', { gameId, mapId });
}

/** Downloads tiles for every map in the game. Progress is reported via the
 *  `tile-download-progress` event ({ gameId, downloaded, total }). */
export async function downloadAllGameTiles(gameId: number): Promise<void> {
  return invoke('download_all_game_tiles', { gameId });
}

export interface TileProgress {
  gameId: number;
  downloaded: number;
  total: number;
}

export interface TileMeta {
  min_zoom: number;
  max_zoom: number;
  extension: string;
  url_template?: string;
}

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