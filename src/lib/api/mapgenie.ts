// Barrel file for the MapGenie API layer.
//
// This used to be one long file. For the modularity assignment we split it by
// topic into games.ts / assets.ts / tiles.ts, and re-export everything here so
// that all the existing `import { ... } from '../lib/api/mapgenie'` lines keep
// working unchanged. (A "barrel" is just a file whose only job is to re-export
// other modules from one convenient place.)

export * from './games';
export * from './assets';
export * from './tiles';
