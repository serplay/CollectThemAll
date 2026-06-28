mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            commands::mapgenie::download_map_tiles,
            commands::mapgenie::download_all_game_tiles,
            commands::mapgenie::ensure_tile_meta
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
