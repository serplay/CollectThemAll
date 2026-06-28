<script lang="ts">
  // Presentational component: the list of buttons that switch between a game's maps.
  //
  // Cybersecurity / software-engineering note: this component is "dumb" on purpose.
  // It only receives data and reports clicks back through the `onSelect` callback —
  // it never changes state itself. Keeping components that only display things
  // separate from the ones that change things makes the data flow easy to follow
  // (and easy to audit), which is the whole point of the modularity assignment.
  import type { Map as GameMap } from '../lib/types/mapgenie';

  let {
    maps,
    activeMapId,
    onSelect,
  }: {
    maps: GameMap[];
    activeMapId: number | undefined;
    onSelect: (map: GameMap) => void;
  } = $props();
</script>

<div class="map-switcher">
  {#each maps as map (map.id)}
    <button
      class="map-link"
      class:selected={activeMapId === map.id}
      onclick={() => onSelect(map)}
    >
      {map.title}
    </button>
  {/each}
</div>

<style>
  .map-switcher {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .map-link {
    background: transparent;
    border: 1px solid #3d3a4f;
    border-radius: 6px;
    color: #f6f6f6;
    padding: 0.5rem 0.75rem;
    text-align: left;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s;
  }

  .map-link:hover {
    background: #2a2540;
  }

  .map-link.selected {
    background: #cf30aa;
    border-color: #cf30aa;
  }
</style>
