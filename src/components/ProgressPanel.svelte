<script lang="ts">
  // Presentational component: the "Progress" box (count, bar, hide-found, reset).
  //
  // Like MapSwitcher, this only displays numbers it is given and reports the user's
  // intent (toggle hide-found / reset all) through callbacks. The actual found-state
  // lives in the parent GameMapView.
  let {
    foundCount,
    totalLocations,
    hideFound,
    onToggleHideFound,
    onClearAll,
  }: {
    foundCount: number;
    totalLocations: number;
    hideFound: boolean;
    onToggleHideFound: () => void;
    onClearAll: () => void;
  } = $props();
</script>

<div class="progress-section">
  <div class="progress-header">
    <h3>Progress</h3>
    <span class="progress-count">{foundCount} / {totalLocations}</span>
  </div>
  <div class="progress-bar-track">
    <div
      class="progress-bar-fill"
      style="width: {totalLocations > 0 ? (foundCount / totalLocations) * 100 : 0}%"
    ></div>
  </div>
  <div class="progress-actions">
    <label class="hide-found-toggle">
      <input type="checkbox" checked={hideFound} onchange={onToggleHideFound} />
      <span>Hide found</span>
    </label>
    {#if foundCount > 0}
      <button class="clear-btn" onclick={onClearAll}>Reset all</button>
    {/if}
  </div>
</div>

<style>
  .progress-section {
    margin-bottom: 0.25rem;
  }

  .progress-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .progress-header h3 {
    font-size: 0.85rem;
    text-transform: uppercase;
    color: #c0b9c0;
    margin: 0;
  }

  .progress-count {
    font-size: 0.85rem;
    color: #a78bfa;
    font-variant-numeric: tabular-nums;
  }

  .progress-bar-track {
    height: 6px;
    background: #2a2540;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.6rem;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #a78bfa, #cf30aa);
    border-radius: 3px;
    transition: width 0.3s ease;
    min-width: 0;
  }

  .progress-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .hide-found-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: #c0b9c0;
    cursor: pointer;
  }

  .clear-btn {
    background: transparent;
    border: 1px solid #5b3a50;
    border-radius: 4px;
    color: #f87171;
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .clear-btn:hover {
    background: #3b1a30;
  }
</style>
