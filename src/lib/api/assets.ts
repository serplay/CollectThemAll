// API calls about a game's downloaded *assets* (images, location data, icons).
//
// Cybersecurity note: a couple of these functions read files straight off the
// local disk via a path the backend hands us. We only ever ask for files under
// the app's own asset folder, and we check `exists()` before using a path, so we
// fail cleanly instead of trusting a path that might not be there.

import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { exists, readTextFile } from '@tauri-apps/plugin-fs';

/** Trigger the (first-time) download of a game's offline assets. No-ops if cached. */
export async function downloadGameAssets(gameId: number): Promise<void> {
  return invoke('download_game_assets', { gameId });
}

/** Whether a game's offline assets are already fully downloaded (lets the UI skip the
 *  "Downloading map assets" screen on repeat opens). */
export async function gameAssetsReady(gameId: number): Promise<boolean> {
  return invoke<boolean>('game_assets_ready', { gameId });
}

/** Resolve a cached image file to a URL the webview can actually display. */
export async function getLocalImageAssetUrl(gameId: number, filename: string): Promise<string> {
  const path = await invoke<string>('get_game_asset_path', { gameId, filename });
  // Always confirm the file is really there before trusting the path.
  if (!(await exists(path))) throw new Error('Asset not cached yet');
  return convertFileSrc(path);
}

/** Read and parse a map's location-data JSON from disk. */
export async function getMapData(gameId: number, mapId: number): Promise<any> {
  const path = await invoke<string>('get_game_asset_path', {
    gameId,
    filename: `maps/${mapId}.json`,
  });
  const raw = await readTextFile(path);
  return JSON.parse(raw);
}

/** Get the icon URL for a category, or null if that icon was not downloaded. */
export async function getMarkerIconUrl(gameId: number, categoryId: number): Promise<string | null> {
  try {
    return await getLocalImageAssetUrl(gameId, `markers/${categoryId}.png`);
  } catch {
    return null; // icon missing — caller should fall back to a generic pin
  }
}
