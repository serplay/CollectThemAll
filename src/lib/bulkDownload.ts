// Orchestrates downloading several games' assets + tiles one after another.
//
// No UI here on purpose — this is pure logic so GameLibrary (and, in theory, any
// other screen) can drive it and just render whatever progress object comes back.
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Game } from './types/mapgenie';
import { downloadGameAssets, downloadAllGameTiles, type TileProgress } from './api/mapgenie';

export interface BulkProgress {
  totalGames: number;
  completedGames: number;
  currentTitle: string;
  currentPct: number; // tile % (0-100) for the game currently in flight
  failures: { id: number; title: string }[];
}

/** Download assets + tiles for a batch of games, reporting progress as it goes.
 *  Resolves with the final progress state (including any failures) once every
 *  game has been attempted. */
export async function downloadGamesBulk(
  games: Game[],
  onProgress: (p: BulkProgress) => void,
): Promise<BulkProgress> {
  const progress: BulkProgress = {
    totalGames: games.length,
    completedGames: 0,
    currentTitle: games[0]?.title ?? '',
    currentPct: 0,
    failures: [],
  };

  // One shared listener for the whole batch — cheaper than resubscribing per game,
  // and we just filter by gameId to only react to the game currently downloading.
  let activeGameId: number | null = games[0]?.id ?? null;
  const unlisten: UnlistenFn = await listen<TileProgress>('tile-download-progress', (e) => {
    if (e.payload.gameId !== activeGameId) return;
    progress.currentPct = e.payload.total > 0
      ? Math.round((e.payload.downloaded / e.payload.total) * 100)
      : 0;
    onProgress({ ...progress });
  });

  try {
    // Sequential on purpose: each game already fans out to 16 concurrent tile
    // fetches on the backend, so doing several games at once would stack that up
    // into something that looks a lot like a small DoS against the tile CDN. One
    // game at a time is the polite-client choice here.
    for (const game of games) {
      activeGameId = game.id;
      progress.currentTitle = game.title;
      progress.currentPct = 0;
      onProgress({ ...progress });

      try {
        await downloadGameAssets(game.id);
        await downloadAllGameTiles(game.id);
      } catch (err) {
        console.error(`Bulk download failed for game ${game.id}:`, err);
        progress.failures.push({ id: game.id, title: game.title });
        // Continue on error — one bad game (network hiccup, missing map data)
        // shouldn't abort downloads the user already asked for.
      }

      progress.completedGames += 1;
      onProgress({ ...progress });
    }
  } finally {
    unlisten();
  }

  return progress;
}
