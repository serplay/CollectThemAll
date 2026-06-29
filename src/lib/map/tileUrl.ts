// Builds the tile URL template that MapLibre uses to request map tiles.
//
// Cybersecurity / platform note: Tauri exposes our custom `tile://` scheme
// differently depending on the operating system. On Windows (and Android) the
// WebView cannot `fetch()` a raw custom scheme, so Tauri maps it to
// `http://tile.localhost`; elsewhere it stays as `tile://localhost`. MapLibre
// fetches tiles from a Web Worker using `fetch()`, which only accepts standard
// schemes, so we must hand it the platform-correct base URL rather than a
// hard-coded one. Picking the wrong form here just means a blank map.

import { usesHttpTileScheme } from '$lib/platform';

/** Returns the `{z}/{x}/{y}` tile template for a given game+map, per platform. */
export function buildTileUrlTemplate(gameId: number, mapId: number): string {
  const tileBase = usesHttpTileScheme ? 'http://tile.localhost' : 'tile://localhost';
  return `${tileBase}/${gameId}/${mapId}/{z}/{x}/{y}`;
}
