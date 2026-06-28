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
</script>

<div class="library-container">
  <SearchBar bind:value={searchQuery} />

  {#if downloadError}
    <div class="error-banner" transition:fade={{ duration: 200 }}>{downloadError}</div>
  {/if}

  {#if isLoading}
    <div class="loader" out:fade={{ duration: 200 }}>Loading maps...</div>
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
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 1.25rem;
  }

  .game-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    flex: 1 1 180px;
    max-width: 240px;
    transition: transform 0.2s ease;
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

  .loader {
    color: #c0b9c0;
    padding: 3rem;
  }

  .error-banner {
    background: #4c2b33;
    color: #f6f6f6;
    padding: 0.75rem 1.25rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }
</style>