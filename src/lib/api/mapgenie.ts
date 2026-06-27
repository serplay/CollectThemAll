import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { exists } from '@tauri-apps/plugin-fs';
import type { Game } from '../types/mapgenie';

export async function fetchGames(): Promise<Game[]> {
  return invoke<Game[]>('fetch_and_cache_games');
}

export async function getLocalImageAssetUrl(
  gameId: number,
  filename: string,
): Promise<string> {
  const path = await invoke<string>('get_game_asset_path', { gameId, filename });
  const fileExists = await exists(path);
  if (!fileExists) throw new Error('Asset not cached yet');
  return convertFileSrc(path);
}