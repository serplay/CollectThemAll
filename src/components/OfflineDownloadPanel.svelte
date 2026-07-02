<script lang="ts">
  // Presentational-ish component: the "download everything for offline" button in the
  // map sidebar. It owns just enough state to show progress; the actual downloading
  // happens in the Rust backend via the already-existing download_all_game_tiles
  // command, so this file only starts it and listens for progress events.
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { downloadAllGameTiles, type TileProgress } from '../lib/api/tiles';

  let { gameId }: { gameId: number } = $props();

  type Status = 'idle' | 'downloading' | 'done';
  let status = $state<Status>('idle');
  let downloaded = $state(0);
  let total = $state(0);
  let error = $state<string | null>(null);

  let unlisten: UnlistenFn | null = null;

  async function startDownload() {
    // Guard clause: ignore extra clicks while a download is already running, same
    // idea as the downloadingId check in GameLibrary — one download at a time.
    if (status === 'downloading') return;
    status = 'downloading';
    downloaded = 0;
    total = 0;
    error = null;

    try {
      if (!unlisten) {
        unlisten = await listen<TileProgress>('tile-download-progress', (e) => {
          if (e.payload.gameId !== gameId) return; // not our game — ignore
          downloaded = e.payload.downloaded;
          total = e.payload.total;
        });
      }
      await downloadAllGameTiles(gameId);
      status = 'done';
    } catch (err) {
      console.error(`Failed to download all tiles for game ${gameId}:`, err);
      error = 'Download failed. Try again.';
      status = 'idle';
    }
  }

  onDestroy(() => {
    unlisten?.();
  });
</script>

<div class="offline-panel">
  <h3>Offline</h3>
  {#if status === 'idle'}
    <button class="download-btn" onclick={startDownload}>Download all maps for offline</button>
  {:else if status === 'downloading'}
    <div class="progress-bar-track">
      <div
        class="progress-bar-fill"
        style="width: {total > 0 ? (downloaded / total) * 100 : 0}%"
      ></div>
    </div>
    <span class="progress-label">
      {total > 0 ? Math.round((downloaded / total) * 100) : 0}% ({downloaded} / {total})
    </span>
  {:else if status === 'done'}
    <span class="done-label">All maps downloaded for offline use.</span>
  {/if}
  {#if error}
    <span class="error-label">{error}</span>
  {/if}
</div>

<style>
  .offline-panel {
    margin-bottom: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .offline-panel h3 {
    font-size: 0.85rem;
    text-transform: uppercase;
    color: #c0b9c0;
    margin: 0;
  }

  .download-btn {
    background: linear-gradient(135deg, #7c3aed, #cf30aa);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .download-btn:hover {
    opacity: 0.85;
  }

  .progress-bar-track {
    height: 6px;
    background: #2a2540;
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #a78bfa, #cf30aa);
    border-radius: 3px;
    transition: width 0.3s ease;
    min-width: 0;
  }

  .progress-label {
    font-size: 0.8rem;
    color: #c0b9c0;
    font-variant-numeric: tabular-nums;
  }

  .done-label {
    font-size: 0.8rem;
    color: #4ade80;
  }

  .error-label {
    font-size: 0.8rem;
    color: #f87171;
  }
</style>
