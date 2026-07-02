<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getMapData, getMarkerIconUrl, ensureTileMeta } from '../lib/api/mapgenie';
  import type { Game, Map as GameMap } from '../lib/types/mapgenie';
  import { loadFoundIds, toggleFound, clearFound, setFoundIds } from '../lib/stores/foundMarkers';
  import {
    listCustomMarkers,
    addCustomMarker,
    updateCustomMarker,
    deleteCustomMarker,
    type CustomMarker,
  } from '../lib/api/customMarkers';
  import { isTauriMobile } from '../lib/platform';
  // Small, focused helpers extracted from this component for the modularity assignment.
  import { buildLocationGeoJson, buildCustomMarkerGeoJson } from '../lib/map/geojson';
  import { buildTileUrlTemplate } from '../lib/map/tileUrl';
  import { buildMarkerPopupElement, buildCustomMarkerPopupElement } from '../lib/map/popup';
  // Sidebar pieces split into their own presentational components.
  import MapSwitcher from './MapSwitcher.svelte';
  import ProgressPanel from './ProgressPanel.svelte';
  import CategoryFilters from './CategoryFilters.svelte';
  import AddMarkerDialog from './AddMarkerDialog.svelte';
  import OfflineDownloadPanel from './OfflineDownloadPanel.svelte';

  // Identifies this window so we ignore our own broadcasts (we already updated locally).
  let windowLabel = '';
  try {
    // @ts-expect-error — __TAURI_INTERNALS__ is injected by Tauri at runtime
    windowLabel = window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label ?? '';
  } catch { /* not under Tauri */ }

  // The full found-id list rides along in the payload because WebView windows don't share
  // their in-memory localStorage cache live — the receiver can't reliably re-read it.
  type FoundChange = { gameId: number; mapId: number; source: string; ids: number[] };

  let {
    game,
    initialMapId,
    overlay = false,
  }: { game: Game; initialMapId?: number; overlay?: boolean } = $props();

  const initialMap =
    (initialMapId != null ? game.maps.find((m) => m.id === initialMapId) : undefined) ??
    game.maps[0] ??
    null;

  let activeMap = $state<GameMap | null>(initialMap);
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
  let sidebarOpen = $state(false);
  // Per-category location counts for progress tracking
  let categoryLocationCounts = $state<Map<number, number>>(new Map());
  // Our own reference to the GeoJSON data so we can update it reliably
  let geoJsonData: { type: 'FeatureCollection'; features: any[] } = { type: 'FeatureCollection', features: [] };

  // Custom markers: the player's own pinned notes (see lib/api/customMarkers.ts).
  let customMarkersList: CustomMarker[] = [];
  let activeCustomPopup: maplibregl.Popup | null = null;
  let addMarkerDialogOpen = $state(false);
  let addMarkerDialogTitle = $state('');
  let addMarkerDialogDescription = $state('');
  // Set when adding a brand-new marker (pin location); cleared when editing an existing one.
  let pendingMarkerLngLat = $state<{ lng: number; lat: number } | null>(null);
  // Set when editing an existing marker; cleared when adding a new one. Drives whether
  // the dialog shows a Delete button, so it needs to be reactive.
  let editingMarkerId = $state<number | null>(null);

  async function loadMap(map: GameMap) {
    isLoadingMap = true;
    loadError = null;
    activeMap = map;

    // Remember the last-opened map so the in-game overlay (Ctrl+Alt+`) can mirror it.
    // Skipped from the overlay itself so it never overrides the main window's choice.
    if (!overlay) {
      try {
        localStorage.setItem('cta:lastMap', JSON.stringify({ gameId: game.id, mapId: map.id }));
      } catch { /* localStorage unavailable — non-fatal */ }
      // Broadcast to the overlay window so it re-syncs live. The storage event isn't
      // reliable across Tauri webviews, so this Tauri event is the primary signal.
      emit('cta:map-changed', { gameId: game.id, mapId: map.id }).catch(() => {});
    }

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

    // Load found state from SQLite.
    foundIds = await loadFoundIds(game.id, map.id);

    // Load the player's own custom markers for this map.
    customMarkersList = await listCustomMarkers(game.id, map.id);

    const catList = await Promise.all(
      [...seen.entries()].map(async ([id, label]) => ({
        id,
        label,
        iconUrl: await getMarkerIconUrl(game.id, id),
      }))
    );
    categories = catList;
    visibleCategoryIds = new Set(catList.map((c) => c.id));

    // Platform-correct tile URL template (see lib/map/tileUrl.ts for the why).
    const tileUrlTemplate = buildTileUrlTemplate(game.id, map.id);

    mapInstance = new maplibregl.Map({
      container: mapContainer,
      style: { version: 8, sources: {}, layers: [] },
      center: [initLng, initLat],
      zoom: initZoom,
      minZoom: mapMinZoom,
      maxZoom: mapMaxZoom,
      renderWorldCopies: false,
      // This is a flat 2D collectible map — no need for bearing/pitch, and right-click
      // drag would otherwise fight with our right-click "Add marker" context menu.
      dragRotate: false,
      pitchWithRotate: false,
      touchPitch: false,
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

      // Build GeoJSON features from all locations, including found state.
      geoJsonData = buildLocationGeoJson(locations, foundIds);

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

        const coords = (feature.geometry as any).coordinates.slice();
        // Build the popup DOM in lib/map/popup.ts; we keep the state change here
        // (toggleFound + rebroadcast) and just hand the builder a callback.
        const popupEl = buildMarkerPopupElement(
          {
            title: locTitle,
            isFound,
            categoryLabel: catLabel,
            categoryIconUrl: catIconUrl,
            description: rawDescription,
            media: mediaItems,
          },
          async () => {
            if (!activeMap) return false;
            const [updated, nowFound] = await toggleFound(game.id, activeMap.id, locId);
            foundIds = updated;
            updateFoundState();
            broadcastFoundChange();
            return nowFound;
          },
        );

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

      // Custom markers: rendered as plain circles (no icon asset needed) so they're
      // visually distinct from the downloaded MapGenie location markers.
      mapInstance.addSource('custom-markers', {
        type: 'geojson',
        data: buildCustomMarkerGeoJson(customMarkersList),
      });
      mapInstance.addLayer({
        id: 'custom-markers-layer',
        type: 'circle',
        source: 'custom-markers',
        paint: {
          'circle-radius': 7,
          'circle-color': '#cf30aa',
          'circle-stroke-width': 2,
          'circle-stroke-color': '#f0ecff',
        },
      });

      mapInstance.on('click', 'custom-markers-layer', (e) => {
        if (!mapInstance || !e.features || e.features.length === 0) return;
        const feature = e.features[0];
        const id = feature.properties?.id;
        const marker = customMarkersList.find((m) => m.id === id);
        if (!marker) return;

        if (activeCustomPopup) {
          activeCustomPopup.remove();
          activeCustomPopup = null;
        }

        const coords = (feature.geometry as any).coordinates.slice();
        const popupEl = buildCustomMarkerPopupElement(
          { title: marker.title, description: marker.description },
          () => {
            activeCustomPopup?.remove();
            activeCustomPopup = null;
            openEditMarkerDialog(marker);
          },
          async () => {
            activeCustomPopup?.remove();
            activeCustomPopup = null;
            await deleteCustomMarker(marker.id);
            await refreshCustomMarkers();
            broadcastCustomMarkersChange();
          },
        );

        activeCustomPopup = new maplibregl.Popup({ offset: 14, closeOnClick: true, maxWidth: '280px' })
          .setLngLat(coords)
          .setDOMContent(popupEl)
          .addTo(mapInstance);
      });

      mapInstance.on('mouseenter', 'custom-markers-layer', () => {
        if (mapInstance) mapInstance.getCanvas().style.cursor = 'pointer';
      });
      mapInstance.on('mouseleave', 'custom-markers-layer', () => {
        if (mapInstance) mapInstance.getCanvas().style.cursor = '';
      });

      // Desktop: right-click anywhere on the map opens "Add marker" at that spot.
      // Mobile has no right-click, so it gets a press-and-hold gesture instead
      // (wired up in onMount via touch listeners on the map container).
      mapInstance.on('contextmenu', (e) => {
        e.preventDefault();
        openAddMarkerDialog(e.lngLat);
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

  async function handleClearAll() {
    if (!activeMap) return;
    await clearFound(game.id, activeMap.id);
    foundIds = new Set();
    updateFoundState();
    broadcastFoundChange();
  }

  /** Notify the other window (main ↔ overlay) that this map's found set changed, sending the
   *  full id list so the receiver can apply it directly. `source` lets us ignore our own echo. */
  function broadcastFoundChange() {
    if (!activeMap) return;
    emit('cta:found-changed', {
      gameId: game.id,
      mapId: activeMap.id,
      source: windowLabel,
      ids: [...foundIds],
    } satisfies FoundChange).catch(() => {});
  }

  /** Apply a found set pushed from the other window: update local state, persist it to
   *  SQLite (both windows share the same database file), and refresh the map + progress UI. */
  async function applyFoundChange(ids: number[]) {
    if (!activeMap) return;
    foundIds = new Set(ids);
    await setFoundIds(game.id, activeMap.id, foundIds);
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

  // ── Custom markers (user-created pins with notes) ─────────────────────────

  /** Re-fetch this map's custom markers from SQLite and push them into the map source. */
  async function refreshCustomMarkers() {
    if (!activeMap) return;
    customMarkersList = await listCustomMarkers(game.id, activeMap.id);
    const source = mapInstance?.getSource('custom-markers') as maplibregl.GeoJSONSource | undefined;
    source?.setData(buildCustomMarkerGeoJson(customMarkersList));
  }

  /** Opens the dialog to create a brand-new marker at the given map-coordinate. */
  function openAddMarkerDialog(lngLat: { lng: number; lat: number }) {
    pendingMarkerLngLat = { lng: lngLat.lng, lat: lngLat.lat };
    editingMarkerId = null;
    addMarkerDialogTitle = '';
    addMarkerDialogDescription = '';
    addMarkerDialogOpen = true;
  }

  /** Opens the dialog pre-filled to edit an existing marker. */
  function openEditMarkerDialog(marker: CustomMarker) {
    pendingMarkerLngLat = null;
    editingMarkerId = marker.id;
    addMarkerDialogTitle = marker.title;
    addMarkerDialogDescription = marker.description;
    addMarkerDialogOpen = true;
  }

  async function handleMarkerDialogSave(title: string, description: string) {
    if (!activeMap) return;
    if (editingMarkerId != null) {
      await updateCustomMarker(editingMarkerId, title, description);
    } else if (pendingMarkerLngLat) {
      await addCustomMarker(
        game.id,
        activeMap.id,
        pendingMarkerLngLat.lat,
        pendingMarkerLngLat.lng,
        title,
        description,
      );
    }
    addMarkerDialogOpen = false;
    await refreshCustomMarkers();
    broadcastCustomMarkersChange();
  }

  async function handleMarkerDialogDelete() {
    if (editingMarkerId != null) {
      await deleteCustomMarker(editingMarkerId);
    }
    addMarkerDialogOpen = false;
    await refreshCustomMarkers();
    broadcastCustomMarkersChange();
  }

  function handleMarkerDialogCancel() {
    addMarkerDialogOpen = false;
  }

  /** Notify the other window that this map's custom markers changed, mirroring
   *  broadcastFoundChange — the receiver just re-fetches the (small) marker list. */
  function broadcastCustomMarkersChange() {
    if (!activeMap) return;
    emit('cta:custom-markers-changed', {
      gameId: game.id,
      mapId: activeMap.id,
      source: windowLabel,
    }).catch(() => {});
  }

  // ── Mobile long-press → "Add marker" (desktop uses right-click instead) ───
  // MapLibre has no built-in long-press event, so we track touchstart/move/end
  // on the map container ourselves: a 1s hold that doesn't move more than a
  // small threshold counts as a long-press; any movement past the threshold
  // (a pan) or lifting early cancels it.
  const LONG_PRESS_MS = 1000;
  const LONG_PRESS_MOVE_THRESHOLD_PX = 10;
  let longPressTimer: ReturnType<typeof setTimeout> | null = null;
  let longPressStart: { x: number; y: number } | null = null;

  function cancelLongPress() {
    if (longPressTimer != null) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
    longPressStart = null;
  }

  function handleTouchStart(e: TouchEvent) {
    if (e.touches.length !== 1) {
      cancelLongPress();
      return;
    }
    const t = e.touches[0];
    longPressStart = { x: t.clientX, y: t.clientY };
    longPressTimer = setTimeout(() => {
      if (!longPressStart || !mapInstance) return;
      const rect = mapContainer.getBoundingClientRect();
      const lngLat = mapInstance.unproject([
        longPressStart.x - rect.left,
        longPressStart.y - rect.top,
      ]);
      longPressStart = null;
      openAddMarkerDialog(lngLat);
    }, LONG_PRESS_MS);
  }

  function handleTouchMove(e: TouchEvent) {
    if (!longPressStart || e.touches.length !== 1) {
      cancelLongPress();
      return;
    }
    const t = e.touches[0];
    const dx = t.clientX - longPressStart.x;
    const dy = t.clientY - longPressStart.y;
    if (Math.hypot(dx, dy) > LONG_PRESS_MOVE_THRESHOLD_PX) {
      cancelLongPress();
    }
  }

  let unlistenFound: UnlistenFn | null = null;
  let unlistenCustomMarkers: UnlistenFn | null = null;

  // Push a sentinel history entry when the sidebar opens so that Android's
  // hardware back gesture closes the drawer rather than navigating away.
  // The sentinel carries { ctaDrawer: true } so we can identify it in popstate.
  function openSidebar() {
    sidebarOpen = true;
    history.pushState({ ctaDrawer: true }, '');
  }

  function closeSidebar() {
    sidebarOpen = false;
    // If the top history entry is our sentinel, pop it so subsequent back
    // gestures navigate normally (e.g. back to the library).
    if (history.state?.ctaDrawer) {
      history.back();
    }
  }

  function onPopState(e: PopStateEvent) {
    // Android back gesture pops the history stack. If it pops our drawer
    // sentinel, intercept it to close the sidebar instead of navigating.
    if (e.state?.ctaDrawer) {
      sidebarOpen = false;
    }
  }

  onMount(() => {
    if (activeMap) loadMap(activeMap);

    window.addEventListener('popstate', onPopState);

    // Mobile has no right-click, so the press-and-hold gesture is its "Add marker" trigger.
    if (isTauriMobile) {
      mapContainer.addEventListener('touchstart', handleTouchStart, { passive: true });
      mapContainer.addEventListener('touchmove', handleTouchMove, { passive: true });
      mapContainer.addEventListener('touchend', cancelLongPress);
      mapContainer.addEventListener('touchcancel', cancelLongPress);
    }

    // Two-way live sync of found markers between the main window and the overlay.
    listen<FoundChange>('cta:found-changed', (e) => {
      const p = e.payload;
      if (p.source === windowLabel) return; // ignore our own broadcast
      if (!activeMap || p.gameId !== game.id || p.mapId !== activeMap.id) return;
      applyFoundChange(p.ids);
    })
      .then((fn) => { unlistenFound = fn; })
      .catch(() => { /* not under Tauri */ });

    // Same pattern for custom markers: the receiver just re-fetches the list.
    listen<{ gameId: number; mapId: number; source: string }>('cta:custom-markers-changed', (e) => {
      const p = e.payload;
      if (p.source === windowLabel) return;
      if (!activeMap || p.gameId !== game.id || p.mapId !== activeMap.id) return;
      refreshCustomMarkers();
    })
      .then((fn) => { unlistenCustomMarkers = fn; })
      .catch(() => { /* not under Tauri */ });
  });

  onDestroy(() => {
    window.removeEventListener('popstate', onPopState);
    if (isTauriMobile) {
      mapContainer.removeEventListener('touchstart', handleTouchStart);
      mapContainer.removeEventListener('touchmove', handleTouchMove);
      mapContainer.removeEventListener('touchend', cancelLongPress);
      mapContainer.removeEventListener('touchcancel', cancelLongPress);
    }
    unlistenFound?.();
    unlistenCustomMarkers?.();
    mapInstance?.remove();
  });
</script>

<div class="map-page" class:overlay>
  <aside class="sidebar" class:open={sidebarOpen}>
    {#if !overlay}
      <a href="/" class="back-link">← Back to library</a>
    {/if}
    <h2 class="game-title">{game.title}</h2>

    <MapSwitcher maps={game.maps} activeMapId={activeMap?.id} onSelect={loadMap} />

    <hr />

    <!-- Progress overview -->
    <ProgressPanel
      foundCount={foundIds.size}
      {totalLocations}
      {hideFound}
      onToggleHideFound={toggleHideFound}
      onClearAll={handleClearAll}
    />

    <hr />

    <!-- Available on every platform (desktop and mobile alike) — the tile fetch
         itself already runs on the trusted Rust side, so there's no extra risk in
         exposing the button everywhere; it's purely a frontend UI decision. -->
    <OfflineDownloadPanel gameId={game.id} />

    <hr />

    <CategoryFilters
      {categories}
      {visibleCategoryIds}
      {categoryLocationCounts}
      {foundInCategory}
      {isLoadingMap}
      onToggleCategory={toggleCategory}
    />
  </aside>

  <main class="map-area" bind:this={mapContainer}>
    {#if isLoadingMap}
      <div class="status">Loading map…</div>
    {:else if loadError}
      <div class="status error">{loadError}</div>
    {/if}
    {#if !overlay}
      <button class="sidebar-toggle" onclick={() => sidebarOpen ? closeSidebar() : openSidebar()} aria-label="Toggle map controls">☰</button>
    {/if}
  </main>

  {#if sidebarOpen}
    <div class="sidebar-backdrop" role="presentation" onclick={() => closeSidebar()}></div>
  {/if}

  {#if addMarkerDialogOpen}
    <AddMarkerDialog
      initialTitle={addMarkerDialogTitle}
      initialDescription={addMarkerDialogDescription}
      onSave={handleMarkerDialogSave}
      onDelete={editingMarkerId != null ? handleMarkerDialogDelete : undefined}
      onCancel={handleMarkerDialogCancel}
    />
  {/if}
</div>

<style>
  .map-page {
    display: flex;
    height: 100vh;
    width: 100%;
  }

  /* In the overlay window the map fills its (bar-offset) container, not the viewport. */
  .map-page.overlay {
    height: 100%;
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

  /* The map list, progress box and category filters now live in their own
     components (MapSwitcher / ProgressPanel / CategoryFilters), each carrying its
     own scoped styles. */

  hr {
    border-color: #3d3a4f;
    margin: 1.25rem 0;
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

  :global(.popup-custom-actions) {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  :global(.popup-custom-actions button) {
    flex: 1;
    border: none;
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  :global(.popup-edit) {
    background: linear-gradient(135deg, #7c3aed, #cf30aa);
    color: #fff;
  }

  :global(.popup-delete) {
    background: transparent;
    border: 1px solid #5b3a50;
    color: #f87171;
  }

  :global(.popup-custom-actions button:hover) {
    opacity: 0.85;
  }

  @keyframes popup-spin {
    to { transform: rotate(360deg); }
  }

  /* Desktop: toggle and backdrop are invisible — pure CSS, no JS branching needed */
  .sidebar-toggle {
    display: none;
  }

  .sidebar-backdrop {
    display: none;
  }

  @media (max-width: 600px) {
    /* Fixed positioning pulls the sidebar out of the flex row so the map fills
       the full viewport. The slide distance is exactly the sheet height, so
       translateY(100%) fully hides it off-screen — no magic number needed. */
    .sidebar {
      position: fixed;
      bottom: 0;
      left: 0;
      right: 0;
      width: 100%;
      height: 75vh;
      border-radius: 16px 16px 0 0;
      /* Cubic-bezier matches iOS sheet spring feel — the higher initial velocity
         then deceleration reads as physical rather than mechanical. */
      transform: translateY(100%);
      transition: transform 0.3s cubic-bezier(0.32, 0.72, 0, 1);
      z-index: 100;
      /* env() only resolves non-zero when viewport-fit=cover is set in <meta>.
         This keeps sidebar content above the iPhone home indicator. */
      padding-bottom: calc(1.25rem + env(safe-area-inset-bottom));
    }

    .sidebar.open {
      transform: translateY(0);
    }

    .sidebar-toggle {
      display: flex;
      position: absolute;
      bottom: calc(1rem + env(safe-area-inset-bottom));
      right: 1rem;
      width: 44px;
      height: 44px;
      align-items: center;
      justify-content: center;
      z-index: 10;
      background: rgba(22, 19, 41, 0.9);
      border: 1px solid rgba(167, 139, 250, 0.4);
      border-radius: 12px;
      color: #a78bfa;
      font-size: 1.25rem;
      cursor: pointer;
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
    }

    /* Backdrop captures taps outside the sheet and blocks map touch events
       while the sidebar is open — without this, panning would leak through. */
    .sidebar-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      z-index: 99;
      background: rgba(0, 0, 0, 0.5);
    }

    /* Keep MapLibre's attribution control above the home indicator */
    :global(.maplibregl-ctrl-bottom-right),
    :global(.maplibregl-ctrl-bottom-left) {
      margin-bottom: env(safe-area-inset-bottom);
    }
  }
</style>
