mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::mapgenie::fetch_and_cache_games,
            commands::mapgenie::get_game_asset_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}