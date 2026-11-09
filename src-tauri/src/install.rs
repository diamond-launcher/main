//! Download & installation engine: metadata -> Java runtime -> verify -> download -> finalize.
use crate::download::{self, Job, Progress};
use crate::error::{AppError, Result};
use crate::java;
use crate::manifest;
use crate::meta::{overlay, AssetIndex, VersionMeta};
use crate::mods;
use crate::paths::{self, GamePaths};
use crate::plan;
use crate::rules::Env;
use crate::settings::Settings;
use crate::state::{AppState, InstallGuard};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub version: String,
    /// metadata | java | verify | download | finalize | done
    pub phase: &'static str,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub files_done: u64,
    pub files_total: u64,
}

pub type Emit = Arc<dyn Fn(InstallProgress) + Send + Sync>;

struct Reporter {
    emit: Emit,
    version: String,
}

impl Reporter {
    fn phase(&self, phase: &'static str) {
        (self.emit)(InstallProgress {
            version: self.version.clone(),
            phase,
            bytes_done: 0,
            bytes_total: 0,
            files_done: 0,
            files_total: 0,
        });
    }
}

fn read_index(paths: &GamePaths, meta: &VersionMeta) -> Result<AssetIndex> {
    let file = paths.asset_index_file(meta.asset_index_id());
    let bytes = std::fs::read(&file).map_err(|_| AppError::new("not_installed", "Asset index is missing."))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Fast structural check used right before launching.
pub fn is_installed(paths: &GamePaths, meta: &VersionMeta, env: &Env) -> bool {
    if !paths.installed_marker(&meta.id).is_file() || !paths.natives(&meta.id).is_dir() {
        return false;
    }
    let libs = plan::resolve_libraries(meta, env, paths);
    let mut jobs = libs.jobs;
    match plan::client_job(meta, paths) {
        Ok(j) => jobs.push(j),
        Err(_) => return false,
    }
    jobs.extend(plan::log_config_job(meta, paths));
    jobs.iter().all(download::is_present) && paths.asset_index_file(meta.asset_index_id()).is_file()
}

pub struct Loaded {
    pub meta: VersionMeta,
    pub index: AssetIndex,
    /// Directory passed to Fabric when launching with mods.
    pub mods_dir: Option<std::path::PathBuf>,
}

pub async fn load_installed(paths: &GamePaths, id: &str, env: &Env, settings: &Settings) -> Result<Loaded> {
    paths::validate_version_id(id)?;
    let mut meta = manifest::read_local_meta(paths, id)?;
    if !is_installed(paths, &meta, env) {
        return Err(AppError::new("not_installed", format!("Version {id} needs to be installed or repaired.")));
    }
    let mut mods_dir = None;
    if !mods::enabled(settings, id).is_empty() {
        let outdated = || AppError::new("mods_outdated", "Mods need to be updated for this version.");
        let state = mods::read_state(paths, id)
            .filter(|s| s.signature == mods::signature(settings, id))
            .ok_or_else(outdated)?;
        if let Some(profile_id) = &state.profile_id {
            let profile = manifest::read_local_meta(paths, profile_id).map_err(|_| outdated())?;
            meta = overlay(profile, meta);
            let dir = paths.mods_dir(id);
            if !is_installed(paths, &meta, env) || !state.files.iter().all(|f| dir.join(f).is_file()) {
                return Err(outdated());
            }
            mods_dir = Some(dir);
        }
    }
    let index = read_index(paths, &meta)?;
    Ok(Loaded { meta, index, mods_dir })
}

/// Installs (or repairs) a version. `deep` re-hashes every existing file.
pub async fn install(emit: Emit, state: &AppState, id: &str, deep: bool) -> Result<()> {
    paths::validate_version_id(id)?;
    let _guard = InstallGuard::acquire(state, id)
        .ok_or_else(|| AppError::new("busy", "Another installation is already in progress."))?;
    if state.game.lock().map(|g| g.is_some()).unwrap_or(false) {
        return Err(AppError::new("busy", "Close the running game before installing."));
    }

    let settings = Settings::load();
    let paths = settings.game_paths();
    let env = Env::current();
    let reporter = Reporter { emit: emit.clone(), version: id.to_string() };
    let http = &state.http;

    reporter.phase("metadata");
    let vanilla = manifest::ensure_meta(http, &paths, id).await?;
    let mods_plan = mods::prepare(http, &paths, id, &settings, deep).await;
    let meta = match &mods_plan.profile {
        Some(profile) => overlay(profile.clone(), vanilla),
        None => vanilla,
    };
    let index_job = plan::index_job(&meta, &paths);
    let missing = download::pending(vec![index_job], true).await?;
    if !missing.is_empty() {
        download::download_all(http, missing, Arc::new(Progress::default())).await?;
    }
    let index = read_index(&paths, &meta)?;

    let libs = plan::resolve_libraries(&meta, &env, &paths);
    let mut jobs: Vec<Job> = vec![plan::client_job(&meta, &paths)?];
    jobs.extend(libs.jobs.iter().cloned());
    jobs.extend(mods_plan.jobs.iter().cloned());
    jobs.extend(plan::log_config_job(&meta, &paths));
    jobs.extend(plan::asset_jobs(&index, &paths));

    let mut runtime = None;
    if settings.java_path.is_none() {
        reporter.phase("java");
        runtime = java::runtime_plan(http, &paths, meta.java_component()).await?;
        if let Some(rt) = &runtime {
            jobs.extend(rt.jobs.iter().cloned());
        }
    }

    reporter.phase("verify");
    let todo = download::pending(jobs, deep).await?;
    let bytes_total: u64 = todo.iter().filter_map(|j| j.size).sum();
    let files_total = todo.len() as u64;

    if !todo.is_empty() {
        let progress = Arc::new(Progress::default());
        let ticker = {
            let progress = progress.clone();
            let emit = emit.clone();
            let version = id.to_string();
            tokio::spawn(async move {
                loop {
                    emit(InstallProgress {
                        version: version.clone(),
                        phase: "download",
                        bytes_done: progress.bytes.load(Ordering::Relaxed),
                        bytes_total,
                        files_done: progress.files.load(Ordering::Relaxed),
                        files_total,
                    });
                    tokio::time::sleep(Duration::from_millis(120)).await;
                }
            })
        };
        let result = download::download_all(http, todo, progress).await;
        ticker.abort();
        result?;
    }

    reporter.phase("finalize");
    let natives_dir = paths.natives(id);
    let archives = libs.natives.clone();
    let index_id = meta.asset_index_id().to_string();
    let paths_clone = paths.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        plan::extract_natives(&archives, &natives_dir)?;
        plan::materialize_legacy_assets(&index, &index_id, &paths_clone)
    })
    .await??;
    if let Some(rt) = &runtime {
        java::finalize_runtime(rt)?;
    }
    if settings.java_path.is_none() && java::resolve(None, &meta, &paths).is_err() {
        return Err(AppError::new("java_missing", format!("Java {} could not be set up.", meta.required_java_major())));
    }
    mods::finish(&paths, id, &mods_plan)?;
    std::fs::write(paths.installed_marker(id), env!("CARGO_PKG_VERSION"))?;

    reporter.phase("done");
    Ok(())
}
