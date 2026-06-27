<script lang="ts">
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, scale } from 'svelte/transition';
  import { fetchGames, getLocalImageAssetUrl } from '../lib/api/mapgenie';
  import type { Game } from '../lib/types/mapgenie';
  import SearchBar from './SearchBar.svelte';

  let games = $state<Game[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);

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
</script>

<div class="library-container">
  <SearchBar bind:value={searchQuery} />

  {#if isLoading}
    <div class="loader" out:fade={{ duration: 200 }}>Loading maps...</div>
  {:else}
    <div class="grid-layout">
      {#each filteredGames as game, i (game.id)}
        <div
          class="game-tile"
          in:fade={{ duration: 300, delay: i * 30 }}
          out:scale={{ duration: 150, start: 0.85 }}
          animate:flip={{ duration: 350 }}
        >
          {#await getLocalImageAssetUrl(game.id, 'logo.jpg')}
            <div class="img-placeholder"></div>
          {:then assetUrl}
            <img src={assetUrl} alt={game.title} loading="lazy" />
          {:catch}
            <img src={game.logo} alt={game.title} loading="lazy" />
          {/await}
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

  .game-tile img,
  .img-placeholder {
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    object-fit: cover;
    background: #161329;
  }

  .img-placeholder {
    animation: pulse 1.4s ease-in-out infinite;
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
</style>