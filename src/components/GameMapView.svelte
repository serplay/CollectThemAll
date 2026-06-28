<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { getMapData, getMarkerIconUrl, ensureTileMeta } from '../lib/api/mapgenie';
  import type { Game, Map as GameMap } from '../lib/types/mapgenie';
  import { loadFoundIds, toggleFound, clearFound } from '../lib/stores/foundMarkers';

  let { game }: { game: Game } = $props();

  let activeMap = $state<GameMap | null>(game.maps[0] ?? null);
  let mapInstance: maplibregl.Map | null = null;
  let mapContainer: HTMLElement;
  let categories = $state<{ id: number; label: string; iconUrl: string | null }[]>([]);
  let visibleCategoryIds = $state<Set<number>>(new Set());
  let isLoadingMap = $state(true);
  let loadError = $state<string | null>(null);
  let foundIds = $state<Set<number>>(new Set());
  let hideFound = $state(false);
  let totalLocations = $state(0);
  let activePopup: maplibregl.Popup | null = null;
  // Per-category location counts for progress tracking
  let categoryLocationCounts = $state<Map<number, number>>(new Map());
  // Our own reference to the GeoJSON data so we can update it reliably
  let geoJsonData: { type: 'FeatureCollection'; features: any[] } = { type: 'FeatureCollection', features: [] };

  // Some older MapGenie games (e.g. RDR2) store location lat/lng in a large-scale coordinate
  // space (lat ~-52 to 82, lng ~-164 to 135) rather than directly in the bounds space
  // ([-1.4, 0, 0, 1.4]). Detect by checking whether a sample latitude sits outside bounds.
  function buildCoordNormalizer(
    locations: any[],
    bounds: [number, number, number, number]
  ): (lat: number, lng: number) => [number, number] {
    const [west, south, east, north] = bounds;
    const sampleLat = parseFloat(locations[0]?.latitude ?? '0');

    if (sampleLat >= south - 0.1 && sampleLat <= north + 0.1) {
      // Already in bounds space — use directly
      return (lat, lng) => [lng, lat];
    }

    // Legacy coordinate system — compute linear normalization to bounds space
    const lats = locations.map((l: any) => parseFloat(l.latitude));
    const lngs = locations.map((l: any) => parseFloat(l.longitude));
    const latMin = lats.reduce((a: number, b: number) => Math.min(a, b));
    const latMax = lats.reduce((a: number, b: number) => Math.max(a, b));
    const lngMin = lngs.reduce((a: number, b: number) => Math.min(a, b));
    const lngMax = lngs.reduce((a: number, b: number) => Math.max(a, b));
    const lngSpan = east - west;
    const latSpan = north - south;

    return (lat, lng) => {
      const normLng = ((lng - lngMin) / (lngMax - lngMin)) * lngSpan + west;
      const normLat = ((lat - latMin) / (latMax - latMin)) * latSpan + south;
      return [normLng, normLat];
    };
  }

  async function loadMap(map: GameMap) {
    isLoadingMap = true;
    loadError = null;
    activeMap = map;

    if (mapInstance) {
      mapInstance.remove();
      mapInstance = null;
    }

    let mapJsonData: any;
    let tileMeta: { min_zoom: number; max_zoom: number; extension: string };

    // Only ensure tile *metadata* (zoom range + CDN template) — no bulk download. The map
    // opens immediately and the tile:// protocol handler streams/caches tiles on demand
    // (low-zoom overview first, then detail as it arrives or as you zoom).
    try {
      tileMeta = await ensureTileMeta(game.id, map.id);
    } catch (err) {
      loadError = `Failed to load tile info: ${err}`;
      isLoadingMap = false;
      return;
    }

    try {
      mapJsonData = await getMapData(game.id, map.id);
    } catch (err) {
      loadError = `Failed to load map data: ${err}`;
      isLoadingMap = false;
      return;
    }

    const bounds = mapJsonData.styles?.mapStyle?.bounds as
      | [number, number, number, number]
      | undefined;
    if (!bounds) {
      loadError = 'Map bounds not found in map data.';
      isLoadingMap = false;
      return;
    }
    const [west, south, east, north] = bounds;
    const lngSpan = east - west;
    const latSpan = north - south;

    // Build category list from locations (dedup by category_id)
    const locations: any[] = mapJsonData.locations ?? [];
    const seen = new Map<number, string>();
    const catCounts = new Map<number, number>();
    for (const loc of locations) {
      if (!seen.has(loc.category_id)) seen.set(loc.category_id, loc.title);
      catCounts.set(loc.category_id, (catCounts.get(loc.category_id) ?? 0) + 1);
    }
    categoryLocationCounts = catCounts;
    totalLocations = locations.length;
    const toMapCoords = buildCoordNormalizer(locations, bounds);

    // Load found state from localStorage
    foundIds = loadFoundIds(game.id, map.id);

    const catList = await Promise.all(
      [...seen.entries()].map(async ([id, label]) => ({
        id,
        label,
        iconUrl: await getMarkerIconUrl(game.id, id),
      }))
    );
    categories = catList;
    visibleCategoryIds = new Set(catList.map((c) => c.id));

    // MapGenie stores tiles at true Web Mercator zoom levels — use them directly.
    const wmMinZoom = tileMeta.min_zoom;
    const wmMaxZoom = tileMeta.max_zoom;

    // Tauri exposes custom URI schemes differently per platform: on Windows/Android the
    // scheme is mapped to http://<scheme>.localhost (WebView2 doesn't support raw custom
    // schemes via fetch), elsewhere it's <scheme>://localhost. MapLibre fetches tiles from
    // a WebWorker using fetch(), which only accepts standard schemes, so we must use the
    // platform-correct form rather than a hardcoded "tile://".
    const tileBase = navigator.userAgent.includes('Windows')
      ? 'http://tile.localhost'
      : 'tile://localhost';
    const tileUrlTemplate = `${tileBase}/${game.id}/${map.id}/{z}/{x}/{y}`;

    mapInstance = new maplibregl.Map({
      container: mapContainer,
      style: { version: 8, sources: {}, layers: [] },
      bounds: [
        [west, south],
        [east, north],
      ],
    });

    // Provide a transparent 1×1 fallback for any image MapLibre can't find,
    // so the symbol layer renders silently rather than spamming console warnings.
    mapInstance.on('styleimagemissing', (e) => {
      if (mapInstance && !mapInstance.hasImage(e.id)) {
        mapInstance.addImage(e.id, { width: 1, height: 1, data: new Uint8Array(4) });
      }
    });

    mapInstance.on('load', async () => {
      if (!mapInstance) return;

      mapInstance.addSource('game-tiles', {
        type: 'raster',
        tiles: [tileUrlTemplate],
        tileSize: 256,
        minzoom: wmMinZoom,
        maxzoom: wmMaxZoom,
      });
      mapInstance.addLayer({ id: 'tiles-layer', type: 'raster', source: 'game-tiles' });

      // Register marker icons with MapLibre's image registry
      for (const cat of categories) {
        if (!cat.iconUrl) continue;
        try {
          const img = await mapInstance.loadImage(cat.iconUrl);
          mapInstance.addImage(`marker-${cat.id}`, img.data);
        } catch {
          // Missing icon is non-fatal — marker will render without an icon
        }
      }

      // Build GeoJSON features from all locations, including found state
      geoJsonData = {
        type: 'FeatureCollection',
        features: locations
          .filter((loc: any) => loc.latitude && loc.longitude)
          .map((loc: any) => {
            const [normLng, normLat] = toMapCoords(
              parseFloat(loc.latitude),
              parseFloat(loc.longitude)
            );
            return {
              type: 'Feature' as const,
              geometry: { type: 'Point' as const, coordinates: [normLng, normLat] },
              properties: {
                id: loc.id,
                title: loc.title,
                category_id: loc.category_id,
                found: foundIds.has(loc.id) ? 1 : 0,
              },
            };
          }),
      };

      mapInstance.addSource('locations', {
        type: 'geojson',
        data: geoJsonData,
      });

      // Layer for unfound markers (full opacity)
      mapInstance.addLayer({
        id: 'markers-layer',
        type: 'symbol',
        source: 'locations',
        layout: {
          'icon-image': ['concat', 'marker-', ['to-string', ['get', 'category_id']]],
          'icon-size': 0.5,
          'icon-allow-overlap': true,
          'icon-anchor': 'bottom',
        },
        paint: {
          'icon-opacity': [
            'case',
            ['==', ['get', 'found'], 1],
            0.5,
            1,
          ],
        },
      });

      // Click handler to toggle found state on markers
      mapInstance.on('click', 'markers-layer', (e) => {
        if (!mapInstance || !e.features || e.features.length === 0) return;
        const feature = e.features[0];
        const locId = feature.properties?.id;
        const locTitle = feature.properties?.title ?? 'Unknown';
        if (!locId || !activeMap) return;

        const isFound = foundIds.has(locId);

        // Close any existing popup
        if (activePopup) {
          activePopup.remove();
          activePopup = null;
        }

        const coords = (feature.geometry as any).coordinates.slice();
        const popupEl = document.createElement('div');
        popupEl.className = 'marker-popup';
        popupEl.innerHTML = `
          <div class="popup-title">${locTitle}</div>
          <button class="popup-toggle ${isFound ? 'found' : ''}">
            ${isFound ? '✓ Found — click to unmark' : 'Mark as found'}
          </button>
        `;

        const btn = popupEl.querySelector('.popup-toggle')!;
        btn.addEventListener('click', () => {
          if (!activeMap) return;
          const [updated, nowFound] = toggleFound(game.id, activeMap.id, locId);
          foundIds = updated;
          updateFoundState();

          // Update popup button
          btn.className = `popup-toggle ${nowFound ? 'found' : ''}`;
          btn.textContent = nowFound ? '✓ Found — click to unmark' : 'Mark as found';
        });

        activePopup = new maplibregl.Popup({ offset: 25, closeOnClick: true })
          .setLngLat(coords)
          .setDOMContent(popupEl)
          .addTo(mapInstance);
      });

      // Change cursor on marker hover
      mapInstance.on('mouseenter', 'markers-layer', () => {
        if (mapInstance) mapInstance.getCanvas().style.cursor = 'pointer';
      });
      mapInstance.on('mouseleave', 'markers-layer', () => {
        if (mapInstance) mapInstance.getCanvas().style.cursor = '';
      });

      applyFilter();
      isLoadingMap = false;
    });

    mapInstance.on('error', (e) => {
      // Suppress tile 404s (expected for out-of-bounds/sparse maps)
      if (e.error?.status === 404) return;
      console.warn('MapLibre error:', e);
    });
  }

  function applyFilter() {
    if (!mapInstance?.getLayer('markers-layer')) return;
    const ids = [...visibleCategoryIds];

    const categoryFilter: maplibregl.FilterSpecification = [
      'in',
      ['get', 'category_id'],
      ['literal', ids],
    ];

    if (hideFound) {
      mapInstance.setFilter('markers-layer', [
        'all',
        categoryFilter,
        ['==', ['get', 'found'], 0],
      ]);
    } else {
      mapInstance.setFilter('markers-layer', categoryFilter);
    }
  }

  /** Update the 'found' property on all GeoJSON features to reflect current state. */
  function updateFoundState() {
    if (!mapInstance) return;
    const source = mapInstance.getSource('locations') as maplibregl.GeoJSONSource | undefined;
    if (!source) return;

    for (const f of geoJsonData.features) {
      f.properties.found = foundIds.has(f.properties.id) ? 1 : 0;
    }
    source.setData(geoJsonData);
    applyFilter();
  }

  function toggleCategory(id: number) {
    const next = new Set(visibleCategoryIds);
    next.has(id) ? next.delete(id) : next.add(id);
    visibleCategoryIds = next;
    applyFilter();
  }

  function toggleHideFound() {
    hideFound = !hideFound;
    applyFilter();
  }

  function handleClearAll() {
    if (!activeMap) return;
    clearFound(game.id, activeMap.id);
    foundIds = new Set();
    updateFoundState();
  }

  /** Count how many found markers belong to a specific category. */
  function foundInCategory(catId: number): number {
    if (foundIds.size === 0) return 0;
    let count = 0;
    for (const f of geoJsonData.features) {
      if (f.properties.category_id === catId && foundIds.has(f.properties.id)) {
        count++;
      }
    }
    return count;
  }

  onMount(() => {
    if (activeMap) loadMap(activeMap);
  });

  onDestroy(() => mapInstance?.remove());
</script>

<div class="map-page">
  <aside class="sidebar">
    <a href="/" class="back-link">← Back to library</a>
    <h2 class="game-title">{game.title}</h2>

    <div class="map-switcher">
      {#each game.maps as map (map.id)}
        <button
          class="map-link"
          class:selected={activeMap?.id === map.id}
          onclick={() => loadMap(map)}
        >
          {map.title}
        </button>
      {/each}
    </div>

    <hr />

    <!-- Progress overview -->
    <div class="progress-section">
      <div class="progress-header">
        <h3>Progress</h3>
        <span class="progress-count">{foundIds.size} / {totalLocations}</span>
      </div>
      <div class="progress-bar-track">
        <div
          class="progress-bar-fill"
          style="width: {totalLocations > 0 ? (foundIds.size / totalLocations) * 100 : 0}%"
        ></div>
      </div>
      <div class="progress-actions">
        <label class="hide-found-toggle">
          <input type="checkbox" checked={hideFound} onchange={toggleHideFound} />
          <span>Hide found</span>
        </label>
        {#if foundIds.size > 0}
          <button class="clear-btn" onclick={handleClearAll}>Reset all</button>
        {/if}
      </div>
    </div>

    <hr />

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
            onchange={() => toggleCategory(cat.id)}
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
  </aside>

  <main class="map-area" bind:this={mapContainer}>
    {#if isLoadingMap}
      <div class="status">Loading map…</div>
    {:else if loadError}
      <div class="status error">{loadError}</div>
    {/if}
  </main>
</div>

<style>
  .map-page {
    display: flex;
    height: 100vh;
    width: 100%;
  }

  .sidebar {
    width: 260px;
    flex-shrink: 0;
    background: #161329;
    padding: 1.25rem;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .back-link {
    display: inline-block;
    color: #a78bfa;
    text-decoration: none;
    font-size: 0.8rem;
    margin-bottom: 0.6rem;
    transition: color 0.15s;
  }

  .back-link:hover {
    color: #c4b5fd;
  }

  .game-title {
    font-size: 1.1rem;
    margin-bottom: 1rem;
  }

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

  hr {
    border-color: #3d3a4f;
    margin: 1.25rem 0;
  }

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

  /* ── Progress section ── */
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

  .map-area {
    flex: 1;
    position: relative;
    background: #010201;
  }

  .status {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #c0b9c0;
    font-size: 0.95rem;
    pointer-events: none;
    z-index: 1;
  }

  .status.error {
    color: #f87171;
  }

  /* ── Marker popup styles (global, not scoped) ── */
  :global(.marker-popup) {
    text-align: center;
    min-width: 140px;
  }

  :global(.popup-title) {
    font-weight: 600;
    font-size: 0.85rem;
    margin-bottom: 0.4rem;
    color: #1a1a2e;
  }

  :global(.popup-toggle) {
    display: inline-block;
    background: #7c3aed;
    color: #fff;
    border: none;
    border-radius: 4px;
    padding: 0.3rem 0.7rem;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  :global(.popup-toggle:hover) {
    background: #6d28d9;
  }

  :global(.popup-toggle.found) {
    background: #16a34a;
  }

  :global(.popup-toggle.found:hover) {
    background: #15803d;
  }
</style>
