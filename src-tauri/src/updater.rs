use crate::error::{AppError, Result};
use crate::state::AppState;
use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

const REPO: &str = "diamond-launcher/main";
const INSTALLER_SUFFIX: &str = "setup.exe";
const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// Progress events are emitted at most this often while downloading.
const PROGRESS_EMIT_INTERVAL_MS: u64 = 200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    /// Release notes from the GitHub release.
    pub notes: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub bytes_done: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    /// Release notes body.
    body: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    /// GitHub reports this as `sha256:<hex>` for uploaded assets.
    digest: Option<String>,
}

#[tauri::command]
pub async fn check_update(state: State<'_, AppState>) -> Result<Option<UpdateInfo>> {
    let release = fetch_latest(&state.http).await?;
    // `releases/latest` never returns prereleases, so stable users never see them.
    if !is_newer(&release.tag_name, CURRENT) || installer_asset(&release).is_none() {
        return Ok(None);
    }
    Ok(Some(UpdateInfo {
        version: strip_v(&release.tag_name).to_string(),
        notes: release.body.trim().to_string(),
    }))
}

/// Downloads the latest installer with progress, verifies its checksum, then hands
/// off to a helper that runs the installer silently and relaunches the app.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    if let Some(reason) = busy_reason(&state) {
        return Err(AppError::new("busy", reason));
    }

    let release = fetch_latest(&state.http).await?;
    if !is_newer(&release.tag_name, CURRENT) {
        return Err(AppError::new("update", "Diamond Launcher is already up to date."));
    }
    let asset = installer_asset(&release)
        .ok_or_else(|| AppError::new("update", "The latest release has no installer attached."))?;

    let response = state
        .http
        .get(&asset.browser_download_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(github_error)?;
    let bytes_total = response.content_length().unwrap_or(0);

    let dir = std::env::temp_dir().join("DiamondLauncher-update");
    tokio::fs::create_dir_all(&dir).await?;
    let installer = dir.join("DiamondLauncher-update-setup.exe");
    let mut file = tokio::fs::File::create(&installer).await.map_err(|e| {
        AppError::new("update", format!("Could not stage the update: {e}"))
    })?;

    let mut bytes_done: u64 = 0;
    let mut last_emit = Instant::now();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(github_error)?;
        file.write_all(&chunk)
            .await
            .map_err(|e| AppError::new("update", format!("Failed to write the update: {e}")))?;
        bytes_done += chunk.len() as u64;
        if last_emit.elapsed().as_millis() >= PROGRESS_EMIT_INTERVAL_MS as u128 || bytes_done == bytes_total {
            let _ = app.emit(
                "update-progress",
                UpdateProgress { bytes_done, bytes_total },
            );
            last_emit = Instant::now();
        }
    }
    file.flush().await.map_err(|e| AppError::new("update", format!("Failed to stage the update: {e}")))?;
    drop(file);

    // Verify the downloaded asset against GitHub's SHA-256 digest before installing anything.
    let bytes = tokio::fs::read(&installer).await.map_err(|e| {
        AppError::new("update", format!("Failed to read the staged update: {e}"))
    })?;
    if let Some(expected) = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        let actual = hex::encode(Sha256::digest(&bytes));
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&installer).await;
            return Err(AppError::new("update", "The downloaded update failed checksum verification."));
        }
    }

    let exe = std::env::current_exe()?;
    // Tauri NSIS bundles install per-user to %LOCALAPPDATA%\Programs\Diamond Launcher.
    let installed = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|p| std::path::PathBuf::from(p).join(r#"Programs\Diamond Launcher\diamond-launcher.exe"#));
    let installed_quoted = installed
        .as_ref()
        .map(|p| ps_quote(p))
        .unwrap_or_else(|| ps_quote(&exe));
    let script = format!(
        "Wait-Process -Id {pid} -ErrorAction SilentlyContinue; \
         Start-Process -FilePath {installer} -ArgumentList '/S' -Wait; \
         $installed = {installed_quoted}; \
         if (Test-Path $installed) {{ Start-Process -FilePath $installed }} \
         else {{ Start-Process -FilePath {exe} }}",
        pid = std::process::id(),
        installer = ps_quote(&installer),
        exe = ps_quote(&exe),
    );
    spawn_hidden_powershell(&script)?;
    app.exit(0);
    Ok(())
}

fn busy_reason(state: &AppState) -> Option<&'static str> {
    if state.game.lock().map(|g| g.is_some()).unwrap_or(false) {
        return Some("Close Minecraft before updating.");
    }
    if state.installing.lock().map(|i| i.is_some()).unwrap_or(false) {
        return Some("Wait for the current install to finish before updating.");
    }
    None
}

async fn fetch_latest(client: &reqwest::Client) -> Result<Release> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let response = client
        .get(url)
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(github_error)?;
    response.json::<Release>().await.map_err(github_error)
}

fn installer_asset(release: &Release) -> Option<&Asset> {
    release
        .assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(INSTALLER_SUFFIX))
}

fn github_error(e: reqwest::Error) -> AppError {
    AppError::new("update", format!("Could not check GitHub for updates: {e}"))
}

/// Returns true when the tag carries a pre-release or build suffix (e.g. `0.5.0-alpha`).
fn is_prerelease_tag(tag: &str) -> bool {
    let trimmed = strip_v(tag);
    let core_end = trimmed.find(|c: char| c == '-' || c == '+').unwrap_or(trimmed.len());
    core_end < trimmed.len()
}

fn strip_v(tag: &str) -> &str {
    tag.trim().trim_start_matches(|c: char| c == 'v' || c == 'V')
}

/// Parses `MAJOR.MINOR.PATCH`, ignoring a leading `v` and any pre-release/build suffix.
fn parse_version(s: &str) -> Option<[u64; 3]> {
    let core = strip_v(s).split(|c: char| c == '-' || c == '+').next()?;
    let mut parts = core.split('.');
    let mut out = [0u64; 3];
    for slot in out.iter_mut() {
        *slot = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(out)
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => {
            // A stable release is always newer than a prerelease with the same core version.
            a > b || (a == b && is_prerelease_tag(current) && !is_prerelease_tag(candidate))
        }
        _ => false,
    }
}

fn ps_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

fn spawn_hidden_powershell(script: &str) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_with_and_without_v_prefix() {
        assert_eq!(parse_version("v0.2.0"), Some([0, 2, 0]));
        assert_eq!(parse_version("1.10.3"), Some([1, 10, 3]));
        assert_eq!(parse_version("V1.0.0-beta.1"), Some([1, 0, 0]));
    }

    #[test]
    fn rejects_malformed_versions() {
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
    }

    #[test]
    fn compares_numerically_not_lexically() {
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.2.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn stable_release_is_newer_than_prerelease_current() {
        assert!(is_newer("v0.5.0", "0.5.0-alpha"));
        assert!(!is_newer("v0.5.0", "0.5.0"));
    }

    #[test]
    fn selects_installer_asset_only() {
        let release = Release {
            tag_name: "v0.2.0".into(),
            body: String::new(),
            assets: vec![
                Asset { name: "notes.txt".into(), browser_download_url: "https://x/notes".into(), digest: None },
                Asset {
                    name: "DiamondLauncher-0.2.0-x64-setup.exe".into(),
                    browser_download_url: "https://x/setup".into(),
                    digest: None,
                },
            ],
        };
        assert_eq!(installer_asset(&release).map(|a| a.name.as_str()), Some("DiamondLauncher-0.2.0-x64-setup.exe"));
    }

    #[test]
    fn quotes_paths_for_powershell() {
        let quoted = ps_quote(Path::new(r"C:\Users\O'Neil\setup.exe"));
        assert_eq!(quoted, r"'C:\Users\O''Neil\setup.exe'");
    }
}
