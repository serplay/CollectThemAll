<script lang="ts">
  // Presentational component: shows progress for a running (or just-finished) bulk
  // download. Like ProgressPanel/MapSwitcher, it only displays the BulkProgress
  // object it's handed — GameLibrary owns the actual downloadGamesBulk call.
  import type { BulkProgress } from '../lib/bulkDownload';

  let { progress, running }: { progress: BulkProgress; running: boolean } = $props();

  let overallPct = $derived(
    progress.totalGames > 0
      ? Math.round(((progress.completedGames + progress.currentPct / 100) / progress.totalGames) * 100)
      : 0,
  );
</script>

<div class="bulk-bar">
  {#if running}
    <div class="bulk-status">
      Downloading "{progress.currentTitle}" ({progress.completedGames + 1} of {progress.totalGames}) — {progress.currentPct}%
    </div>
  {:else}
    <div class="bulk-status">
      Downloaded {progress.completedGames} of {progress.totalGames} games.
    </div>
  {/if}
  <div class="progress-bar-track">
    <div class="progress-bar-fill" style="width: {overallPct}%"></div>
  </div>
  {#if progress.failures.length > 0}
    <div class="failures">
      Failed: {progress.failures.map((f) => f.title).join(', ')}
    </div>
  {/if}
</div>

<style>
  .bulk-bar {
    width: 100%;
    max-width: 1400px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background: #161329;
    border-radius: 8px;
  }

  .bulk-status {
    font-size: 0.85rem;
    color: #c0b9c0;
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

  .failures {
    font-size: 0.8rem;
    color: #f87171;
  }
</style>
