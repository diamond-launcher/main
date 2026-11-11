use crate::error::{AppError, Result};
use crate::install;
use crate::launch;
use crate::manifest;
use crate::paths;
use crate::settings::Settings;
use crate::state::AppState;
use crate::system::{self, SystemInfo};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    /// "release" or "snapshot"
    pub kind: String,
    pub release_time: String,
    pub installed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<VersionInfo>,
    /// True when the list comes from the on-disk cache because Mojang was unreachable.
    pub offline: bool,
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    system::detect()
}

#[tauri::command]
pub fn get_settings() -> Settings {
    let mut settings = Settings::load();
    // Adopt changes the player made in the in-game config screen.
    if crate::hud::import_ingame(&mut settings) {
        let _ = settings.save();
    }
    settings
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<Settings> {
    let (total, _) = system::memory_mb();
    let mut next = settings.normalized(total)?;
    // Selection and play history are owned by the backend, never overwritten by the settings form.
    let current = Settings::load();
    next.selected_version = current.selected_version;
    next.last_played = current.last_played;
    let hud_changed = next.hud != current.hud;
    next.save()?;
    // Only rewrite the mod config when the HUD was edited here. Otherwise leave an
    // in-game file untouched so `import_ingame` can pick it up before the next launch.
    if hud_changed {
        let _ = crate::hud::write_config(&next.game_paths(), &next.hud);
    }
    Ok(next)
}

#[tauri::command]
pub fn select_version(id: String, show_snapshots: bool) -> Result<()> {
    paths::validate_version_id(&id)?;
    let mut settings = Settings::load();
    settings.selected_version = Some(id);
    settings.show_snapshots = show_snapshots;
    settings.save()
}

#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> Result<VersionList> {
    let game_paths = Settings::load().game_paths();
    let (manifest, offline) = manifest::load(&state.http).await?;
    let versions = manifest
        .versions
        .into_iter()
        .filter(|v| v.kind == "release" || v.kind == "snapshot")
        .map(|v| VersionInfo {
            installed: game_paths.installed_marker(&v.id).is_file(),
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect();
    Ok(VersionList { latest_release: manifest.latest.release, latest_snapshot: manifest.latest.snapshot, versions, offline })
}

#[tauri::command]
pub async fn install_version(app: AppHandle, state: State<'_, AppState>, id: String, verify: bool) -> Result<()> {
    let emit: install::Emit = std::sync::Arc::new(move |p| {
        let _ = app.emit("install-progress", p);
    });
    install::install(emit, &state, &id, verify).await
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    launch::launch(&app, &state, &id).await
}

#[tauri::command]
pub fn stop_game(state: State<'_, AppState>) -> Result<()> {
    launch::stop(&state)
}

#[tauri::command]
pub fn list_mod_catalog() -> &'static [crate::mods::CatalogEntry] {
    crate::mods::CATALOG
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub developer: &'static str,
    pub website: &'static str,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        developer: "den3zx",
        website: "https://github.com/diamond-launcher/main",
    }
}

#[tauri::command]
pub fn mods_status(id: String) -> Result<Option<crate::mods::ModsState>> {
    paths::validate_version_id(&id)?;
    Ok(crate::mods::read_state(&Settings::load().game_paths(), &id))
}

#[tauri::command]
pub fn clear_mod_state(id: String) -> Result<()> {
    paths::validate_version_id(&id)?;
    let paths = Settings::load().game_paths();
    let state_file = paths.mods_state_file(&id);
    if state_file.exists() {
        std::fs::remove_file(state_file)?;
    }
    // Also remove downloaded mod jars
    let mods_dir = paths.mods_dir(&id);
    if mods_dir.exists() {
        std::fs::remove_dir_all(mods_dir)?;
    }
    Ok(())
}

#[tauri::command]
pub fn is_game_running(state: State<'_, AppState>) -> Result<Option<String>> {
    let slot = state.game.lock().map_err(|_| AppError::new("internal", "State unavailable"))?;
    Ok(slot.as_ref().map(|h| h.version.clone()))
}
