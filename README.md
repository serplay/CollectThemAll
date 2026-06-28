<p align="center">
  <img src="./static/banner.png" alt="CollectThemAll Banner" width="100%">
</p>

<h1 align="center">CTA — CollectThemAll</h1>

<p align="center">
  A blazing-fast, native desktop overlay for tracking game collectibles on interactive maps.
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.0-24C8DB?logo=tauri&logoColor=white">
  <img alt="Svelte" src="https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white">
  <img alt="License" src="https://img.shields.io/badge/License-MIT-green">
  <img alt="Platforms" src="https://img.shields.io/badge/Desktop-Windows%20%7C%20macOS%20%7C%20Linux-blue">
</p>

> 📱 **Mobile version incoming** — the UI and state logic already share a codebase that compiles to Android and iOS via Tauri v2. Native mobile builds are on the roadmap.

## What it does

CollectThemAll (CTA) is a cross-platform companion app for completionists. It pulls
interactive game maps and collectible locations, renders them in a smooth, hardware-accelerated
map view, and lets you check off every chest, shrine, and secret as you find it — **without
alt-tabbing out of your game**.

A global hotkey summons a transparent, always-on-top overlay that floats over the running game
and mirrors whatever map you last opened, so your progress stays in sync wherever you mark it.

## Why it's useful

Most game trackers are Electron apps that ship a full Chromium instance, eating hundreds of
megabytes of RAM and stealing CPU cycles (and frames) from the game you're actually trying to
play. CTA is built on **Tauri v2 + Svelte 5**, compiling to a lightweight native binary that
renders thousands of map markers without dropping system frames.

- **🪶 Featherweight** — a native binary instead of a bundled browser; minimal RAM/CPU footprint while gaming.
- **🎮 In-game overlay** — `Ctrl + Alt + \`` toggles a borderless, always-on-top window that mirrors your active map.
- **⚡ Instant maps, streamed tiles** — a Rust caching proxy serves map tiles from disk and streams higher-res tiles from the CDN on demand, so maps open immediately.
- **📴 Offline first** — game assets, marker data, and your found-location progress are all saved locally.
- **🔍 Filter & track** — toggle marker categories, hide found items, and watch per-category completion progress update live.
- **🔄 Cross-window sync** — mark a location in the main window or the overlay; both stay consistent.

## Screenshots

### In-game overlay

The floating, always-on-top overlay mirrors your active map so you can track collectibles without leaving the game.

<p align="center">
  <img src="./static/overlay.png" alt="In-game overlay" width="80%">
</p>

### Game library

Pick a game to load its maps and collectible data.

<p align="center">
  <img src="./static/game_library.png" alt="Game library" width="80%">
</p>

### Interactive map

Thousands of markers rendered smoothly, with category filters and live completion tracking.

<p align="center">
  <img src="./static/full_map_view.png" alt="Full map view" width="80%">
</p>

<table>
  <tr>
    <td width="50%"><img src="./static/details.png" alt="Marker details popup" width="100%"></td>
    <td width="50%"><img src="./static/progress.png" alt="Per-category progress" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><em>Marker details &amp; found toggle</em></td>
    <td align="center"><em>Per-category completion progress</em></td>
  </tr>
</table>

## Tech stack

| Layer        | Technology                                                        |
| ------------ | ----------------------------------------------------------------- |
| Frontend     | [Svelte 5](https://svelte.dev) (runes) + TypeScript               |
| Map engine   | [MapLibre GL JS](https://maplibre.org) (Web Mercator projection)  |
| Backend      | [Tauri v2](https://tauri.app) (Rust)                              |
| Networking   | `reqwest` + `tokio` async tile fetching with a custom `tile://` protocol |
| Build / dev  | Vite 6 + SvelteKit (static adapter)                               |

## Getting started

### Prerequisites

- **Node.js** v18+ and a package manager (npm, pnpm, or yarn)
- **Rust** toolchain (`cargo`, `rustc`) — install via [rustup](https://rustup.rs)
- **OS build dependencies:**
  - **Windows:** Microsoft C++ Build Tools (MSVC) + WebView2
  - **Linux:** `webkit2gtk` and related packages ([see Tauri prerequisites](https://tauri.app/start/prerequisites/))
  - **macOS:** Xcode Command Line Tools

### Run in development

```bash
# 1. Clone
git clone https://github.com/yourusername/CollectThemAll.git
cd CollectThemAll

# 2. Install frontend dependencies
npm install

# 3. Launch the app with hot reload (frontend + Rust backend)
npm run tauri dev
```

### Usage

1. Launch CTA — the **game library** loads available games.
2. Click a game to download its lightweight assets (markers + location data). Map tiles stream in on demand.
3. Browse the map, toggle marker categories, and click a marker to mark it as **found**.
4. While in a game, press **`Ctrl + Alt + \``** to toggle the floating overlay and keep tracking without leaving the game (press **Esc** to dismiss it).

### Build for production

```bash
npm run tauri build
```

Optimized installers/executables are written to `src-tauri/target/release/bundle/`.

## Project structure

```text
CollectThemAll/
├── src/                       # Svelte 5 frontend
│   ├── components/            # GameLibrary, GameMapView, SearchBar, Background
│   ├── lib/
│   │   ├── api/mapgenie.ts    # Tauri command bindings
│   │   ├── stores/            # found-marker persistence (localStorage)
│   │   └── types/             # shared TypeScript types
│   └── routes/               # SvelteKit routes (library, game/[id], overlay)
└── src-tauri/                # Rust backend
    ├── src/
    │   ├── lib.rs            # window mgmt, global shortcut, tile:// protocol
    │   └── commands/         # asset fetching, tile download/caching
    └── tauri.conf.json       # Tauri configuration
```

## Roadmap

- [ ] Native Android & iOS builds
- [ ] User-defined custom markers and notes
- [ ] Cloud sync of found progress across devices

## Getting help

- **Bugs & feature requests:** open an [issue](../../issues).
- **Questions & ideas:** start a [discussion](../../discussions).

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](docs/CONTRIBUTING.md) before opening a
pull request. Run `npm run check` to type-check the frontend before submitting.

## License

Released under the [MIT License](LICENSE).
