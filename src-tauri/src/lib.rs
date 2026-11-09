mod commands;
mod download;
mod error;
mod hud;
mod install;
mod java;
mod launch;
mod manifest;
mod meta;
mod mods;
mod paths;
mod plan;
mod platform;
mod rules;
mod settings;
mod state;
mod system;
mod updater;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_system_info,
            commands::get_settings,
            commands::save_settings,
            commands::select_version,
            commands::list_versions,
            commands::install_version,
            commands::launch_game,
            commands::stop_game,
            commands::is_game_running,
            commands::list_mod_catalog,
            commands::mods_status,
            commands::clear_mod_state,
            commands::get_app_info,
            updater::check_update,
            updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Diamond Launcher");
}
