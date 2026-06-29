<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { fetchGames, downloadGameAssets } from '../../lib/api/mapgenie';
  import type { Game } from '../../lib/types/mapgenie';
  import GameMapView from '../../components/GameMapView.svelte';

  // This is the always-on-top overlay window shown with Ctrl+Alt+`. It mirrors
  // whatever map the main window last opened. The two windows are separate WebViews
  // that cannot share live memory, so they coordinate through small messages
  // (Tauri events + the storage event) rather than by reaching into each other —
  // a clean trust boundary between the two windows.
  let game = $state<Game | null>(null);
  let mapId = $state<number | undefined>(undefined);
  let status = $state<string>('Loading overlay…');
  let ready = $state(false);

  // Tracks which selection is currently shown so we can ignore no-op reloads.
  let currentGameId: number | null = null;
  let currentMapId: number | null = null;
  let loadToken = 0; // guards against out-of-order async loads

  function hideOverlay() {
    try {
      getCurrentWindow().hide();
    } catch {
      /* not in Tauri (e.g. dev preview in a browser) — ignore */
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') hideOverlay();
  }

  /** Reads cta:lastMap and (re)loads the chosen game/map if it differs from what's shown. */
  async function loadChosen() {
    let chosen: { gameId: number; mapId: number } | null = null;
    try {
      const raw = localStorage.getItem('cta:lastMap');
      if (raw) chosen = JSON.parse(raw);
    } catch {
      /* ignore malformed value */
    }

    if (!chosen) {
      status = 'Open a map in the main window first, then press Ctrl+Alt+` again.';
      ready = false;
      return;
    }

    // Already showing this exact selection — nothing to do (avoids needless remounts).
    if (chosen.gameId === currentGameId && chosen.mapId === currentMapId && ready) return;

    const token = ++loadToken;
    try {
      const games = await fetchGames(); // 12h-cached, not a network re-fetch
      const g = games.find((x) => x.id === chosen!.gameId) ?? null;
      if (!g) {
        status = 'The last-opened game is no longer available.';
        ready = false;
        return;
      }
      await downloadGameAssets(g.id); // no-ops if already cached
      if (token !== loadToken) return; // a newer load superseded this one

      currentGameId = chosen.gameId;
      currentMapId = chosen.mapId;
      mapId = chosen.mapId;
      game = g;
      ready = true;
    } catch (err) {
      console.error('Overlay failed to load map:', err);
      if (token === loadToken) status = 'Failed to load the overlay map.';
    }
  }

  // The main window writes cta:lastMap when you open a different game/map; the storage event
  // fires here (other same-origin window) so the overlay re-syncs live while it's open.
  function onStorage(e: StorageEvent) {
    if (e.key === 'cta:lastMap') loadChosen();
  }

  // Also re-check whenever the overlay regains focus (e.g. shown again via the hotkey after
  // the selection changed while it was hidden, in case a storage event was missed).
  function onFocus() {
    loadChosen();
  }

  let unlisten: (() => void) | null = null;

  onMount(() => {
    window.addEventListener('keydown', onKey);
    window.addEventListener('storage', onStorage);
    window.addEventListener('focus', onFocus);
    // Primary live-sync: the main window emits this when its game/map selection changes.
    listen('cta:map-changed', () => loadChosen())
      .then((fn) => { unlisten = fn; })
      .catch(() => { /* not in Tauri — fall back to storage/focus */ });
    loadChosen();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', onKey);
    window.removeEventListener('storage', onStorage);
    window.removeEventListener('focus', onFocus);
    unlisten?.();
  });
</script>

<div class="overlay-root">
  <div class="overlay-bar" data-tauri-drag-region>
    <span class="overlay-title" data-tauri-drag-region>
      {ready && game ? game.title : 'CollectThemAll'} — Overlay
    </span>
    <span class="overlay-hint" data-tauri-drag-region>
      drag to move · Esc or Ctrl+Alt+` to hide
    </span>
    <button class="overlay-hide" onclick={hideOverlay}>Hide ✕</button>
  </div>

  <div class="overlay-body">
    {#if ready && game}
      {#key `${game.id}:${mapId}`}
        <GameMapView {game} initialMapId={mapId} overlay={true} />
      {/key}
    {:else}
      <div class="overlay-status">{status}</div>
    {/if}
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    background: #0e0b1c;
  }

  .overlay-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100%;
  }

  .overlay-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 32px;
    flex-shrink: 0;
    padding: 0 0.5rem 0 0.85rem;
    background: #161329;
    border-bottom: 1px solid #2a2540;
    user-select: none;
    cursor: grab;
  }

  .overlay-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: #e9d5ff;
    white-space: nowrap;
  }

  .overlay-hint {
    flex: 1;
    font-size: 0.72rem;
    color: #8b85a3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .overlay-hide {
    background: transparent;
    border: 1px solid #3d3a4f;
    border-radius: 5px;
    color: #f6f6f6;
    font-size: 0.72rem;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .overlay-hide:hover {
    background: #cf30aa;
    border-color: #cf30aa;
  }

  .overlay-body {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .overlay-status {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 2rem;
    text-align: center;
    color: #c0b9c0;
    font-size: 0.95rem;
  }
</style>
