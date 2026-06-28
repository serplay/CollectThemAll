<script lang="ts">
  // Presentational component: the per-category filter checkboxes in the sidebar.
  //
  // It is handed the category list plus a couple of lookup helpers (how many are
  // found / how many exist per category) and reports checkbox toggles back to the
  // parent. No state of its own.
  type Category = { id: number; label: string; iconUrl: string | null };

  let {
    categories,
    visibleCategoryIds,
    categoryLocationCounts,
    foundInCategory,
    isLoadingMap,
    onToggleCategory,
  }: {
    categories: Category[];
    visibleCategoryIds: Set<number>;
    categoryLocationCounts: Map<number, number>;
    foundInCategory: (catId: number) => number;
    isLoadingMap: boolean;
    onToggleCategory: (id: number) => void;
  } = $props();
</script>

<div class="filters">
  <h3>Filters</h3>
  {#if categories.length === 0 && !isLoadingMap}
    <p class="no-categories">No categories found.</p>
  {/if}
  {#each categories as cat (cat.id)}
    <label class="filter-item">
      <input
        type="checkbox"
        checked={visibleCategoryIds.has(cat.id)}
        onchange={() => onToggleCategory(cat.id)}
      />
      {#if cat.iconUrl}
        <img src={cat.iconUrl} alt="" class="filter-icon" />
      {/if}
      <span class="filter-label">{cat.label}</span>
      <span class="filter-progress">
        {foundInCategory(cat.id)}/{categoryLocationCounts.get(cat.id) ?? 0}
      </span>
    </label>
  {/each}
</div>

<style>
  .filters h3 {
    font-size: 0.85rem;
    text-transform: uppercase;
    color: #c0b9c0;
    margin-bottom: 0.6rem;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .filter-icon {
    width: 20px;
    height: 20px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .filter-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .filter-progress {
    font-size: 0.75rem;
    color: #a78bfa;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .no-categories {
    font-size: 0.85rem;
    color: #c0b9c0;
  }
</style>
