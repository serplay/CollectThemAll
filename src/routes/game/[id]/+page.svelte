<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { fetchGames, downloadGameAssets, gameAssetsReady } from '../../../lib/api/mapgenie';
  import type { Game } from '../../../lib/types/mapgenie';
  import GameMapView from '../../../components/GameMapView.svelte';
  import Background from '../../../components/Background.svelte';

  let game = $state<Game | null>(null);
  let isLoading = $state(true);
  let isDownloading = $state(false);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    const gameId = Number($page.params.id);
    try {
      const games = await fetchGames(); // served from 12hr cache, not a re-fetch of the list
      game = games.find((g) => g.id === gameId) ?? null;
      if (!game) {
        loadError = 'Game not found.';
        isLoading = false;
        return;
      }

      isLoading = false;

      // Only show the downloading screen the first time; afterwards assets are cached and
      // download_game_assets returns immediately, so open the map without the flash.
      const ready = await gameAssetsReady(game.id);
      if (!ready) {
        isDownloading = true;
        await downloadGameAssets(game.id); // creates assets/{id}/... on first open
        isDownloading = false;
      }
    } catch (err) {
      console.error('Failed to load/download game assets:', err);
      loadError = 'Failed to download game assets.';
      isLoading = false;
      isDownloading = false;
    }
  });
</script>

{#if isLoading}
  <Background />
  <div class="status">
    <a href="/" class="back-link">← Back</a>
    <p>Loading...</p>
  </div>
{:else if isDownloading}
  <Background />
  <div class="status">
    <a href="/" class="back-link">← Back</a>
    <p>Downloading map assets...</p>
  </div>
{:else if loadError}
  <Background />
  <div class="status">
    <a href="/" class="back-link">← Back</a>
    <p>{loadError}</p>
  </div>
{:else if game}
  <GameMapView {game} />
{/if}

<style>
  .status {
    color: #c0b9c0;
    padding: 3rem;
    text-align: center;
    position: relative;
    z-index: 1;
  }

  .status p {
    margin: 0;
  }

  .back-link {
    display: inline-block;
    margin-bottom: 1.5rem;
    color: #a78bfa;
    text-decoration: none;
    font-size: 0.9rem;
    transition: color 0.15s;
  }

  .back-link:hover {
    color: #c4b5fd;
  }
</style>