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

  function simpleMarkdownToHtml(text: string): string {
    return text
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/\n/g, '<br>');
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

    // MapGenie renders with MapLibre/Mapbox GL using *real* Web Mercator: location
    // latitude/longitude are already EPSG:4326 lng/lat, and the tiles are standard XYZ
    // Web Mercator tiles. So we feed raw coordinates straight through — no normalization.
    // The view (center/zoom) and zoom range come from the map config, mirroring MapGenie's
    // own setup: center=[start_lng,start_lat], zoom=initial_zoom-1, min/max zoom one below
    // the tile range (their tile z is offset by 1 from the MapLibre zoom).
    const tileMinZoom = tileMeta.min_zoom;
    const tileMaxZoom = tileMeta.max_zoom;
    const mapMinZoom = Math.max(0, tileMinZoom - 1);
    const mapMaxZoom = Math.max(mapMinZoom, tileMaxZoom - 1);
    const initLng = map.initial_longitude ?? 0;
    const initLat = map.initial_latitude ?? 0;
    const initZoom = Math.max(mapMinZoom, Math.min(mapMaxZoom, (map.initial_zoom ?? tileMinZoom) - 1));

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
      center: [initLng, initLat],
      zoom: initZoom,
      minZoom: mapMinZoom,
      maxZoom: mapMaxZoom,
      renderWorldCopies: false,
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
        minzoom: mapMinZoom,
        maxzoom: tileMaxZoom,
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
            return {
              type: 'Feature' as const,
              geometry: {
                type: 'Point' as const,
                coordinates: [parseFloat(loc.longitude), parseFloat(loc.latitude)],
              },
              properties: {
                id: loc.id,
                title: loc.title,
                category_id: loc.category_id,
                description: loc.description ?? '',
                media: JSON.stringify(loc.media ?? []),
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
        const catId = feature.properties?.category_id;
        const catLabel = categories.find((c) => c.id === catId)?.label ?? '';
        const catIconUrl = categories.find((c) => c.id === catId)?.iconUrl ?? '';
        const rawDescription: string = feature.properties?.description ?? '';
        let mediaItems: { url: string; type: string }[] = [];
        try { mediaItems = JSON.parse(feature.properties?.media ?? '[]'); } catch { /* noop */ }

        // Close any existing popup
        if (activePopup) {
          activePopup.remove();
          activePopup = null;
        }

        const hasImage = mediaItems.length > 0 && mediaItems[0].type === 'image';
        const mediaHtml = hasImage
          ? `<div class="popup-img-wrap">
               <div class="popup-img-spinner" aria-hidden="true"></div>
               <img class="popup-media" src="${mediaItems[0].url}" alt="" />
             </div>`
          : '';
        const descHtml = rawDescription
          ? `<div class="popup-desc">${simpleMarkdownToHtml(rawDescription)}</div>`
          : '';
        const catHtml = catLabel
          ? `<div class="popup-category">${catIconUrl ? `<img src="${catIconUrl}" class="popup-cat-icon" alt="" />` : ''}${catLabel}</div>`
          : '';

        const coords = (feature.geometry as any).coordinates.slice();
        const popupEl = document.createElement('div');
        popupEl.className = 'marker-popup';
        popupEl.innerHTML = `
          ${mediaHtml}
          <div class="popup-body">
            ${catHtml}
            <div class="popup-title">${locTitle}</div>
            ${descHtml}
            <button class="popup-toggle ${isFound ? 'found' : ''}">
              ${isFound ? '✓ Found — click to unmark' : 'Mark as found'}
            </button>
          </div>
        `;

        // Wire up image loading: show spinner until loaded, hide on error
        const img = popupEl.querySelector('.popup-media') as HTMLImageElement | null;
        if (img) {
          const spinner = img.previousElementSibling as HTMLElement | null;
          img.addEventListener('load', () => {
            if (spinner) spinner.style.display = 'none';
            img.style.opacity = '1';
          });
          img.addEventListener('error', () => {
            if (spinner) spinner.style.display = 'none';
            img.style.display = 'none';
          });
        }

        const btn = popupEl.querySelector('.popup-toggle')!;
        btn.addEventListener('click', () => {
          if (!activeMap) return;
          const [updated, nowFound] = toggleFound(game.id, activeMap.id, locId);
          foundIds = updated;
          updateFoundState();

          btn.className = `popup-toggle ${nowFound ? 'found' : ''}`;
          btn.textContent = nowFound ? '✓ Found — click to unmark' : 'Mark as found';
        });

        activePopup = new maplibregl.Popup({ offset: 25, closeOnClick: true, maxWidth: '280px' })
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

  /* Override MapLibre's default popup chrome to match dark theme */
  :global(.maplibregl-popup-content) {
    background: rgba(18, 14, 36, 0.92) !important;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(167, 139, 250, 0.25);
    border-radius: 10px !important;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
    padding: 0 !important;
    overflow: hidden;
    color: #f0ecff;
    font-family: inherit;
    min-width: 200px;
    max-width: 280px;
  }

  :global(.maplibregl-popup-tip) {
    border-top-color: rgba(18, 14, 36, 0.92) !important;
  }

  :global(.maplibregl-popup-close-button) {
    color: #a78bfa !important;
    font-size: 1rem;
    padding: 0.3rem 0.5rem;
    top: 4px;
    right: 4px;
  }

  :global(.marker-popup) {
    width: 100%;
  }

  :global(.popup-img-wrap) {
    position: relative;
    background: #0d0b1e;
    min-height: 72px;
  }

  :global(.popup-img-spinner) {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :global(.popup-img-spinner::after) {
    content: '';
    width: 22px;
    height: 22px;
    border: 2px solid rgba(167, 139, 250, 0.35);
    border-top-color: #a78bfa;
    border-radius: 50%;
    animation: popup-spin 0.7s linear infinite;
  }

  :global(.popup-media) {
    display: block;
    width: 100%;
    max-height: 160px;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.2s;
  }

  :global(.popup-body) {
    padding: 0.75rem;
  }

  :global(.popup-category) {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #a78bfa;
    margin-bottom: 0.3rem;
  }

  :global(.popup-cat-icon) {
    width: 14px;
    height: 14px;
    object-fit: contain;
    flex-shrink: 0;
  }

  :global(.popup-title) {
    font-weight: 700;
    font-size: 0.9rem;
    color: #f0ecff;
    margin-bottom: 0.5rem;
    line-height: 1.3;
  }

  :global(.popup-desc) {
    font-size: 0.78rem;
    color: #c4b5fd;
    line-height: 1.5;
    margin-bottom: 0.65rem;
    max-height: 100px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #4c3a7c transparent;
  }

  :global(.popup-desc strong) {
    color: #e9d5ff;
    font-weight: 600;
  }

  :global(.popup-desc em) {
    color: #d8b4fe;
  }

  :global(.popup-toggle) {
    display: block;
    width: 100%;
    background: linear-gradient(135deg, #7c3aed, #cf30aa);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 0.4rem 0.7rem;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
    text-align: center;
  }

  :global(.popup-toggle:hover) {
    opacity: 0.85;
  }

  :global(.popup-toggle.found) {
    background: linear-gradient(135deg, #16a34a, #15803d);
  }

  @keyframes popup-spin {
    to { transform: rotate(360deg); }
  }
</style>
