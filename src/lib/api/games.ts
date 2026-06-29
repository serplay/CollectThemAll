// API calls about the *games list*.
//
// Cybersecurity note: every function in here ultimately calls Tauri's `invoke`,
// which is the one and only bridge from this (browser-like) frontend into the
// trusted Rust backend. We can only call commands the backend explicitly allows,
// so this file is really a list of "doors" the backend opened for us.

import { invoke } from '@tauri-apps/api/core';
import type { Game } from '../types/mapgenie';

/** Ask the backend for the list of games (served from a 12h cache when possible). */
export async function fetchGames(): Promise<Game[]> {
  return invoke<Game[]>('fetch_and_cache_games');
}
