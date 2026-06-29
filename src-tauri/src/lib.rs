//! Library crate: builds and runs the Tauri desktop application.
//!
//! Cybersecurity studies note: this file is where we wire up the trust boundary
//! between the web UI and the native backend. Two things here are worth a
//! security reviewer's attention: (1) the `invoke_handler` allow-list at the
//! bottom, which is the *only* set of Rust functions the frontend may call, and
//! (2) the custom `tile://` protocol handler, which serves bytes to the webview
//! and therefore must be careful about which paths/URLs it will fetch.

mod commands;

/// Shows the in-game overlay if it's hidden, hides it if it's visible, creating the
/// always-on-top overlay window on first use (so there's no spare window at startup).
/// The overlay loads the `/overlay` route, which mirrors whichever map was last opened
/// in the main window and lets the player mark locations found without leaving the game.
#[cfg(desktop)]
fn toggle_overlay(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("overlay") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
        return;
    }

    // Load the SPA entry (always resolvable in both dev and prod); the root layout detects
    // the "overlay" window label and client-routes to /overlay. Loading "/overlay" directly
    // would work in dev but 404 in the static prod build, so we route on the client instead.
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("CollectThemAll - Overlay")
    .inner_size(1100.0, 720.0)
    .min_inner_size(700.0, 480.0)
    .always_on_top(true)
    .decorations(false)
    .skip_taskbar(true)
    .build();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};
                // Ctrl+Alt+` (backtick) toggles the overlay, even while a game has focus.
                let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Backquote);
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcut(toggle.clone())?
                        .with_handler(move |app, shortcut, event| {
                            if shortcut == &toggle && event.state() == ShortcutState::Pressed {
                                toggle_overlay(app);
                            }
                        })
                        .build(),
                )?;
            }
            Ok(())
        })
        .plugin(tauri_plugin_fs::init())
        // tile://localhost/{game_id}/{map_id}/{z}/{x}/{y}
        // Caching proxy: serves the tile from disk if cached, otherwise fetches it from the
        // CDN on the fly and persists it. This lets the map open instantly and stream in
        // higher-resolution tiles as they arrive. Async so the network fetch never blocks
        // the webview; the work runs on the Tauri runtime and responds when ready.
        .register_asynchronous_uri_scheme_protocol("tile", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn(async move {
                let response = commands::mapgenie::serve_tile_request(&app, &path).await;
                responder.respond(response);
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::mapgenie::fetch_and_cache_games,
            commands::mapgenie::get_game_asset_path,
            commands::mapgenie::download_game_assets,
            commands::mapgenie::game_assets_ready,
            commands::mapgenie::download_map_tiles,
            commands::mapgenie::download_all_game_tiles,
            commands::mapgenie::ensure_tile_meta
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
