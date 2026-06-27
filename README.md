<p align="center">
  <img src="./static/banner.png" alt="CollectThemAll Banner" width="100%">
</p>

# CTA (CollectThemAll)

CTA is a cross-platform desktop overlay and mobile app for tracking game collectibles on custom map grids.

The primary constraint for this project is performance. Standard game trackers built on Electron bundle a full Chromium instance, which causes unnecessary RAM and CPU usage while a game is running. By using Tauri v2 and Svelte 5, CTA compiles to a lightweight native binary that handles thousands of map markers without dropping system frames.

## Tech Stack

* **Frontend:** Svelte 5 (TypeScript)
* **Map Engine:** Leaflet (Configured for CRS.Simple pixel coordinates)
* **Backend:** Tauri v2 (Rust)
* **Storage:** Local file system via Rust standard library

## Core Features

* **Desktop Overlay:** Configured as a transparent, borderless, always-on-top window.
* **Mobile Ready:** UI and state logic share a codebase capable of compiling to Android and iOS via Tauri v2.
* **Zero-VDOM Updates:** Svelte 5 runes handle marker state updates directly in the DOM, preventing lag when filtering large datasets.
* **Offline First:** All coordinate data and user progress is handled and saved locally.

## Setup & Compilation

### Prerequisites

Ensure you have the following installed on your system:

* Node.js (v18+) and your preferred package manager
* Rust toolchain (`cargo`, `rustc`)
* OS-specific build dependencies (MSVC for Windows, `webkit2gtk` for Linux, Xcode CLI tools for macOS)

### Development

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/CTA-CollectThemAll.git
   cd CTA-CollectThemAll
    ```

2. Install frontend dependencies:

    ```bash
    npm install
    ```

3. Initialize the development environment:

    ```bash
    npm run tauri dev
    ```

### Building for Production

To compile an optimized release binary for your host operating system, run:

```bash
npm run tauri build
```

The compiled executables will be generated in `src-tauri/target/release/bundle/`.

## Directory Structure

* `src/` : Frontend application. Contains all Svelte components, state management, and Leaflet initialization.
* `src-tauri/` : Native backend. Contains the Tauri configuration (`tauri.conf.json`), Rust source code for window management, and native system hooks.

## License

This project is licensed under the MIT License.
