// API calls for user-created custom markers (title + notes pinned anywhere on a map).
//
// Backed by SQLite on the Rust side (src-tauri/src/db/userdata.rs) — the frontend never
// runs SQL itself, only the #[tauri::command]s below.

import { invoke } from '@tauri-apps/api/core';

export interface CustomMarker {
  id: number;
  gameId: number;
  mapId: number;
  latitude: number;
  longitude: number;
  title: string;
  description: string;
  color: string | null;
  createdAt: number;
  updatedAt: number;
}

export async function listCustomMarkers(gameId: number, mapId: number): Promise<CustomMarker[]> {
  return invoke<CustomMarker[]>('list_custom_markers', { gameId, mapId });
}

export async function addCustomMarker(
  gameId: number,
  mapId: number,
  latitude: number,
  longitude: number,
  title: string,
  description: string,
): Promise<CustomMarker> {
  return invoke<CustomMarker>('add_custom_marker', {
    gameId,
    mapId,
    latitude,
    longitude,
    title,
    description,
  });
}

export async function updateCustomMarker(
  id: number,
  title: string,
  description: string,
): Promise<CustomMarker | null> {
  return invoke<CustomMarker | null>('update_custom_marker', { id, title, description });
}

export async function deleteCustomMarker(id: number): Promise<void> {
  return invoke('delete_custom_marker', { id });
}
