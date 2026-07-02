<script lang="ts">
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, scale } from 'svelte/transition';
  import { fetchGames, getLocalImageAssetUrl, downloadGameAssets } from '../lib/api/mapgenie';
  import type { Game } from '../lib/types/mapgenie';
  import SearchBar from './SearchBar.svelte';
  import { goto } from '$app/navigation';

  let games = $state<Game[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);
  let downloadingId = $state<number | null>(null);
  let downloadError = $state<string | null>(null);

  // Multi-select mode: lets the user pick several games to bulk-download at once
  // (see BulkDownloadBar / bulkDownload.ts). Kept as its own commit's worth of
  // state/UX so the selection mechanics are reviewable separately from the
  // downloading logic that consumes selectedIds.
  let selectMode = $state(false);
  let selectedIds = $state<Set<number>>(new Set());

  let filteredGames = $derived(
    games.filter(game => game.title.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  onMount(async () => {
    try {
      games = await fetchGames();
    } catch (error) {
      console.error("Failed to load mapgenie games:", error);
    } finally {
      isLoading = false;
    }
  });

  async function handleGameClick(game: Game) {
    // In select mode a tile click toggles selection instead of opening the game —
    // keeps the two interaction modes from fighting over the same click handler.
    if (selectMode) {
      toggleSelected(game.id);
      return;
    }

    // Guard clause: if a download is already running, ignore further clicks. This
    // stops the user from kicking off several downloads at once by clicking around
    // — a simple example of validating state before acting, not just trusting the
    // UI to be in a sensible state.
    if (downloadingId !== null) return;
    downloadingId = game.id;
    downloadError = null;
    try {
      // Only the lightweight per-game assets (markers, location data). Map tiles are NOT
      // bulk-downloaded — they stream in on demand once the map opens, so this stays quick.
      await downloadGameAssets(game.id);
      goto(`/game/${game.id}`);
    } catch (error) {
      console.error(`Failed to download assets for game ${game.id}:`, error);
      downloadError = `Couldn't download "${game.title}". Try again.`;
    } finally {
      downloadingId = null;
    }
  }

  function toggleSelected(id: number) {
    const next = new Set(selectedIds);
    next.has(id) ? next.delete(id) : next.add(id);
    selectedIds = next;
  }

  function toggleSelectMode() {
    selectMode = !selectMode;
    // Leaving select mode should not leave a stale selection lying around.
    if (!selectMode) selectedIds = new Set();
  }

  function selectAll() {
    selectedIds = new Set(filteredGames.map((g) => g.id));
  }

  function cancelSelectMode() {
    selectMode = false;
    selectedIds = new Set();
  }
</script>

<div class="library-container">
  <div class="search-sticky-wrap">
    <SearchBar bind:value={searchQuery} />
  </div>

  <div class="header-actions">
    {#if !selectMode}
      <button class="select-toggle-btn" onclick={toggleSelectMode}>Select</button>
    {:else}
      <button class="select-toggle-btn" onclick={selectAll}>Select all</button>
      <button class="select-toggle-btn" onclick={cancelSelectMode}>Cancel</button>
      <button class="select-toggle-btn primary" disabled={selectedIds.size === 0}>
        Download {selectedIds.size} selected
      </button>
    {/if}
  </div>

  {#if downloadError}
    <div class="error-banner" transition:fade={{ duration: 200 }}>{downloadError}</div>
  {/if}

  {#if isLoading}
    <div class="loader" out:fade={{ duration: 200 }}>
      <span class="loader-spinner"></span>
      <span>Loading game library…</span>
    </div>
  {:else}
    <div class="grid-layout">
      {#each filteredGames as game, i (game.id)}
        <div
          class="game-tile"
          class:is-downloading={downloadingId === game.id}
          role="button"
          tabindex="0"
          onclick={() => handleGameClick(game)}
          onkeydown={(e) => e.key === 'Enter' && handleGameClick(game)}
          in:fade={{ duration: 300, delay: i * 30 }}
          out:scale={{ duration: 150, start: 0.85 }}
          animate:flip={{ duration: 300 }}
        >
          <div class="tile-image-wrap">
            {#await getLocalImageAssetUrl(game.id, 'logo.jpg')}
              <div class="img-placeholder"></div>
            {:then assetUrl}
              <img src={assetUrl} alt={game.title} loading="lazy" />
            {:catch}
              <img src={game.logo} alt={game.title} loading="lazy" />
            {/await}
            {#if downloadingId === game.id}
              <div class="download-overlay" transition:fade={{ duration: 150 }}>
                <span class="spinner"></span>
              </div>
            {/if}
            {#if selectMode && selectedIds.has(game.id)}
              <div class="download-overlay selected-overlay" transition:fade={{ duration: 150 }}>
                <span class="checkmark">✓</span>
              </div>
            {/if}
          </div>
          <div class="tile-title">{game.title}</div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .library-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 100%;
    padding-inline: 1.5rem;
    padding-bottom: 2rem;
    margin: 0 auto;
  }

  .grid-layout {
    width: 100%;
    max-width: 1400px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 1.25rem;
  }

  .game-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    transition: transform 0.2s ease;
    /* Suppress the iOS blue tap flash on non-button interactive elements */
    -webkit-tap-highlight-color: transparent;
    min-height: 44px;
  }

  .game-tile:hover {
    transform: translateY(-4px);
  }

  .game-tile.is-downloading {
    cursor: wait;
  }

  .tile-image-wrap {
    position: relative;
    width: 100%;
  }

  .game-tile img,
  .img-placeholder {
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    object-fit: cover;
    background: #161329;
    display: block;
  }

  .img-placeholder {
    animation: pulse 1.4s ease-in-out infinite;
  }

  .download-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(1, 2, 1, 0.6);
    border-radius: 10px;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-top-color: #cf30aa;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
  }

  .tile-title {
    font-size: 0.9rem;
    text-align: center;
    width: 100%;
    overflow-wrap: break-word;
  }

  .search-sticky-wrap {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .header-actions {
    width: 100%;
    max-width: 1400px;
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .select-toggle-btn {
    background: transparent;
    border: 1px solid #3d3a4f;
    border-radius: 6px;
    color: #f6f6f6;
    padding: 0.4rem 0.75rem;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .select-toggle-btn:hover {
    background: #2a2540;
  }

  .select-toggle-btn.primary {
    background: linear-gradient(135deg, #7c3aed, #cf30aa);
    border: none;
  }

  .select-toggle-btn.primary:hover {
    opacity: 0.85;
  }

  .select-toggle-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .selected-overlay {
    background: rgba(124, 58, 237, 0.55);
  }

  .checkmark {
    color: #fff;
    font-size: 1.5rem;
    font-weight: 700;
  }

  .loader {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    color: #c0b9c0;
    padding: 3rem;
  }

  .loader-spinner {
    width: 36px;
    height: 36px;
    border: 3px solid rgba(255, 255, 255, 0.15);
    border-top-color: #cf30aa;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .error-banner {
    background: #4c2b33;
    color: #f6f6f6;
    padding: 0.75rem 1.25rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  @media (max-width: 600px) {
    .library-container {
      padding-inline: 0.75rem;
    }

    .grid-layout {
      grid-template-columns: repeat(2, 1fr);
      gap: 0.75rem;
    }
  }

  /* Pin the search bar to the top so the user never has to scroll back up to search.
     Two conditions so it triggers on phones in BOTH orientations: portrait phones
     match max-width, landscape phones (wide but very short) match max-height. The
     desktop window is both wide and tall, so it keeps the non-sticky layout. */
  @media (max-width: 600px), (max-height: 600px) {
    .search-sticky-wrap {
      position: sticky;
      /* env(safe-area-inset-top) keeps it below the notch on devices where
         viewport-fit=cover pushes content edge-to-edge. */
      top: env(safe-area-inset-top, 0px);
      z-index: 20;
    }

    /* Trim the SearchBar's built-in 2.5rem gap so the sticky strip stays compact */
    .search-sticky-wrap :global(.search-bar-row) {
      margin-block: 0.6rem;
    }
  }
</style>