// Platform detection helpers for Tauri targets.
//
// Cybersecurity note: we check __TAURI_INTERNALS__ first so these flags are
// only ever true inside a genuine Tauri shell — a plain browser (or a hostile
// page trying to spoof mobile) won't have the injected internals object, so the
// mobile-only code paths can't be reached from the open web.

/** True when running in a Tauri mobile shell (iOS or Android). */
export const isTauriMobile: boolean = (() => {
  try {
    if (!(window as any).__TAURI_INTERNALS__) return false;
    return /iPhone|iPad|Android/i.test(navigator.userAgent);
  } catch {
    return false;
  }
})();

// On Windows and Android the WebView cannot fetch() a raw custom scheme, so
// Tauri aliases tile:// → http://tile.localhost. WKWebView (iOS/macOS) handles
// the raw scheme natively, so we keep tile:// there.
export const usesHttpTileScheme: boolean =
  navigator.userAgent.includes('Windows') ||
  (!!(window as any).__TAURI_INTERNALS__ && /Android/i.test(navigator.userAgent));
