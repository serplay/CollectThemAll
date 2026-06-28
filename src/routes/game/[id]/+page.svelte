<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { fetchGames, downloadGameAssets } from '../../../lib/api/mapgenie';
  import type { Game } from '../../../lib/types/mapgenie';
  import GameMapView from '../../../components/GameMapView.svelte';

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
      isDownloading = true;
      await downloadGameAssets(game.id); // creates assets/{id}/... if missing, no-ops files that already exist
      isDownloading = false;
    } catch (err) {
      console.error('Failed to load/download game assets:', err);
      loadError = 'Failed to download game assets.';
      isLoading = false;
      isDownloading = false;
    }
  });
</script>

{#if isLoading}
  <div class="status">Loading...</div>
{:else if isDownloading}
  <div class="status">Downloading map assets...</div>
{:else if loadError}
  <div class="status">{loadError}</div>
{:else if game}
  <GameMapView {game} />
{/if}

<style>
  .status {
    color: #c0b9c0;
    padding: 3rem;
    text-align: center;
  }
</style>