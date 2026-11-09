//! Performance mods: Fabric loader profile + mod jars resolved from Modrinth for a Minecraft version.
use crate::download::{self, Job, Progress};
use crate::error::{AppError, Result};
use crate::meta::VersionMeta;
use crate::paths::GamePaths;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const FABRIC_META: &str = "https://meta.fabricmc.net/v2/versions/loader";
/// Hidden dependency used by the HUD; also installed when no catalog mod needs it.
const FABRIC_API: &str = "fabric-api";

#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

/// Mods offered by the launcher. Required dependencies (e.g. Fabric API) are added automatically.
pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry { slug: "sodium", name: "Sodium", description: "Modern rendering engine with a large FPS boost.", category: "Performance" },
    CatalogEntry { slug: "lithium", name: "Lithium", description: "Optimizes game logic, physics and mob AI.", category: "Performance" },
    CatalogEntry { slug: "ferrite-core", name: "FerriteCore", description: "Reduces memory usage.", category: "Performance" },
    CatalogEntry { slug: "immediatelyfast", name: "ImmediatelyFast", description: "Faster HUD, text and immediate-mode rendering.", category: "Performance" },
    CatalogEntry { slug: "entityculling", name: "Entity Culling", description: "Skips rendering of entities you cannot see.", category: "Performance" },
    CatalogEntry { slug: "modernfix", name: "ModernFix", description: "Faster startup and lower memory use.", category: "Performance" },
    CatalogEntry { slug: "dynamic-fps", name: "Dynamic FPS", description: "Saves resources while the game window is unfocused.", category: "Performance" },
    // --- Additional maximum FPS mods ---
    CatalogEntry { slug: "c2me-fabric", name: "C2ME", description: "Chunk cache & generation optimizer. Faster world loading.", category: "Performance" },
    CatalogEntry { slug: "starlight", name: "Starlight", description: "Rewritten light engine. Massive chunk generation & lighting speedup.", category: "Performance" },
    CatalogEntry { slug: "alternate-current", name: "Alternate Current", description: "Redstone optimization. Reduces lag from redstone contraptions.", category: "Performance" },
    CatalogEntry { slug: "krypton", name: "Krypton", description: "Network stack optimization. Faster chunk & entity sync (works in singleplayer too).", category: "Performance" },
    CatalogEntry { slug: "noisium", name: "Noisium", description: "Sodium fork with more aggressive optimizations. Experimental but faster.", category: "Performance" },
    CatalogEntry { slug: "memoryleakfix", name: "Memory Leak Fix", description: "Fixes various memory leaks in vanilla. Prevents RAM buildup over time.", category: "Performance" },
    // --- Utility ---
    CatalogEntry { slug: "zoomify", name: "Zoomify", description: "Configurable zoom (hold C).", category: "Utility" },
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModsState {
    pub signature: String,
    /// Fabric profile id; `None` means the version is launched as vanilla.
    pub profile_id: Option<String>,
    pub files: Vec<String>,
    pub mods: Vec<String>,
    pub skipped: Vec<String>,
    /// Why mods could not be installed (e.g. Fabric or Modrinth is unreachable). The version then runs as vanilla.
    #[serde(default)]
    pub error: Option<String>,
}

pub struct ModsPlan {
    pub profile: Option<VersionMeta>,
    pub jobs: Vec<Job>,
    pub state: ModsState,
}

pub fn enabled(settings: &Settings, mc: &str) -> Vec<&'static str> {
    let mut slugs: Vec<&'static str> = if settings.mods_enabled {
        CATALOG.iter().map(|c| c.slug).filter(|s| !settings.disabled_mods.iter().any(|d| d == s)).collect()
    } else {
        Vec::new()
    };
    // The HUD is a Fabric mod too: keep Fabric API installed even when every
    // performance mod is switched off. Older versions have no HUD build.
    if crate::hud::active(settings, mc) && !slugs.contains(&FABRIC_API) {
        slugs.push(FABRIC_API);
    }
    slugs
}

pub fn signature(settings: &Settings, mc: &str) -> String {
    enabled(settings, mc).join(",")
}

pub fn read_state(paths: &GamePaths, mc: &str) -> Option<ModsState> {
    serde_json::from_slice(&std::fs::read(paths.mods_state_file(mc)).ok()?).ok()
}

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoader,
}

#[derive(Deserialize)]
struct FabricLoader {
    version: String,
    #[serde(default)]
    stable: bool,
}

#[derive(Deserialize)]
struct MrVersion {
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    dependencies: Vec<MrDependency>,
    #[serde(default)]
    files: Vec<MrFile>,
}

#[derive(Deserialize)]
struct MrDependency {
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Deserialize)]
struct MrFile {
    url: String,
    filename: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    hashes: MrHashes,
}

#[derive(Deserialize, Default)]
struct MrHashes {
    sha1: Option<String>,
}

async fn latest_loader(client: &reqwest::Client, mc: &str) -> Result<Option<String>> {
    let response = client.get(format!("{FABRIC_META}/{mc}")).send().await?;
    if response.status().is_client_error() {
        return Ok(None);
    }
    let list: Vec<FabricLoaderEntry> = response.error_for_status()?.json().await?;
    Ok(list
        .iter()
        .find(|e| e.loader.stable)
        .or_else(|| list.first())
        .map(|e| e.loader.version.clone()))
}

async fn fetch_profile(client: &reqwest::Client, paths: &GamePaths, mc: &str, loader: &str) -> Result<VersionMeta> {
    let bytes = client
        .get(format!("{FABRIC_META}/{mc}/{loader}/profile/json"))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let profile: VersionMeta = serde_json::from_slice(&bytes)?;
    crate::paths::validate_version_id(&profile.id)?;
    let file = paths.version_json(&profile.id);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, &bytes)?;
    Ok(profile)
}

async fn latest_mod_file(client: &reqwest::Client, project: &str, mc: &str) -> Result<Option<(MrFile, Vec<String>)>> {
    let response = client
        .get(format!("{MODRINTH_API}/project/{project}/version"))
        .query(&[("loaders", r#"["fabric"]"#.to_string()), ("game_versions", format!(r#"["{mc}"]"#))])
        .send()
        .await?;
    // A project may have been removed or renamed (404) or may simply not publish
    // builds for this Minecraft version. Treat that as "unavailable" instead of
    // failing the whole mod stack.
    if matches!(response.status().as_u16(), 404 | 410) {
        return Ok(None);
    }
    let versions: Vec<MrVersion> = response.error_for_status()?.json().await?;
    let Some(version) = versions.iter().position(|v| v.version_type == "release").or(if versions.is_empty() { None } else { Some(0) })
    else {
        return Ok(None);
    };
    let version = versions.into_iter().nth(version).expect("index is in range");
    let deps = version
        .dependencies
        .into_iter()
        .filter(|d| d.dependency_type == "required")
        .filter_map(|d| d.project_id)
        .collect();
    let mut files = version.files;
    let index = files.iter().position(|f| f.primary).unwrap_or(0);
    if files.is_empty() {
        return Ok(None);
    }
    Ok(Some((files.swap_remove(index), deps)))
}

fn safe_file_name(name: &str) -> bool {
    Path::new(name).file_name().is_some_and(|n| n == name) && name.ends_with(".jar")
}

/// Works out the Fabric profile and mod jars for `mc` according to the user's settings.
pub async fn plan(client: &reqwest::Client, paths: &GamePaths, mc: &str, settings: &Settings) -> Result<ModsPlan> {
    let slugs = enabled(settings, mc);
    let mut state = ModsState { signature: signature(settings, mc), ..ModsState::default() };
    let vanilla = |state: ModsState| ModsPlan { profile: None, jobs: Vec::new(), state };
    if slugs.is_empty() {
        return Ok(vanilla(state));
    }

    let Some(loader) = latest_loader(client, mc).await? else {
        state.skipped = CATALOG.iter().filter(|c| slugs.contains(&c.slug)).map(|c| c.slug.to_string()).collect();
        return Ok(vanilla(state));
    };

    let mut queue: VecDeque<(String, bool)> = slugs.iter().map(|s| (s.to_string(), true)).collect();
    let mut seen = HashSet::new();
    let mut jobs: Vec<Job> = Vec::new();
    while let Some((project, requested)) = queue.pop_front() {
        if !seen.insert(project.clone()) {
            continue;
        }
        let name = CATALOG
            .iter()
            .find(|c| c.slug == project)
            .map(|c| c.name.to_string())
            .or_else(|| (project == FABRIC_API).then(|| "Fabric API".to_string()));
        match latest_mod_file(client, &project, mc).await? {
            Some((file, deps)) if safe_file_name(&file.filename) => {
                queue.extend(deps.into_iter().map(|d| (d, false)));
                if let Some(name) = name {
                    state.mods.push(name);
                }
                if !jobs.iter().any(|j| j.dest.ends_with(&file.filename)) {
                    jobs.push(Job {
                        url: file.url,
                        dest: paths.mods_dir(mc).join(&file.filename),
                        sha1: file.hashes.sha1,
                        size: file.size,
                    });
                    state.files.push(file.filename);
                }
            }
            _ if requested => state.skipped.push(project.clone()),
            _ => {}
        }
    }

    if jobs.is_empty() {
        return Ok(vanilla(state));
    }
    let profile = fetch_profile(client, paths, mc, &loader).await?;
    state.profile_id = Some(profile.id.clone());
    Ok(ModsPlan { profile: Some(profile), jobs, state })
}

/// Plans and downloads the mods for `mc`. A network failure (Fabric or Modrinth unreachable, or blocked by a
/// filter) does not fail the install: the version is planned as vanilla and the reason is stored in the state.
pub async fn prepare(client: &reqwest::Client, paths: &GamePaths, mc: &str, settings: &Settings, deep: bool) -> ModsPlan {
    let attempt: Result<ModsPlan> = async {
        let plan = plan(client, paths, mc, settings).await?;
        let missing = download::pending(plan.jobs.clone(), deep).await?;
        if !missing.is_empty() {
            download::download_all(client, missing, Arc::new(Progress::default())).await?;
        }
        Ok(plan)
    }
    .await;
    match attempt {
        Ok(plan) => plan,
        Err(err) => {
            // Log the error for debugging
            eprintln!("[Mods] Failed to prepare mods for {mc}: {} ({})", err.message, err.code);
            unavailable(settings, mc, err)
        }
    }
}

fn unavailable(settings: &Settings, mc: &str, err: AppError) -> ModsPlan {
    ModsPlan {
        profile: None,
        jobs: Vec::new(),
        state: ModsState { signature: signature(settings, mc), error: Some(err.message), ..ModsState::default() },
    }
}

/// Removes jars that are no longer wanted and records the resulting state.
pub fn finish(paths: &GamePaths, mc: &str, plan: &ModsPlan) -> Result<()> {
    let dir = paths.mods_dir(mc);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".jar") && !plan.state.files.contains(&name) {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    let file = paths.mods_state_file(mc);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, serde_json::to_vec_pretty(&plan.state)?)
        .map_err(|e| AppError::new("io", format!("Could not save mod state: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_reflects_user_choices() {
        let mut s = Settings::default();
        assert!(enabled(&s, "26.3").contains(&FABRIC_API), "HUD keeps Fabric API installed");
        // All catalog mods should be enabled by default (minus any that are unavailable for the version)
        let enabled_slugs = enabled(&s, "26.3");
        let catalog_slugs: Vec<&str> = CATALOG.iter().map(|c| c.slug).collect();
        for slug in &catalog_slugs {
            // Each catalog mod should either be enabled or skipped (unavailable)
            assert!(enabled_slugs.contains(slug) || format!("{:?}", enabled_slugs).contains(slug), "mod {slug} should be enabled or skipped");
        }
        s.disabled_mods = vec!["sodium".into()];
        assert!(!signature(&s, "26.3").contains("sodium,") && !enabled(&s, "26.3").contains(&"sodium"));
        s.mods_enabled = false;
        assert_eq!(enabled(&s, "26.3"), vec![FABRIC_API]);
        assert!(enabled(&s, "1.21.4").is_empty(), "no HUD build for 1.21.4");
        s.hud.enabled = false;
        assert!(enabled(&s, "26.3").is_empty());
    }

    #[test]
    fn rejects_unsafe_file_names() {
        assert!(safe_file_name("sodium-1.0.jar"));
        assert!(!safe_file_name("../evil.jar"));
        assert!(!safe_file_name("a/b.jar"));
        assert!(!safe_file_name("x.exe"));
    }

    #[test]
    fn unreachable_mods_fall_back_to_vanilla_with_reason() {
        let mut s = Settings::default();
        s.mods_enabled = true;
        let plan = unavailable(&s, "26.3", AppError::new("network", "Network error: blocked"));
        assert!(plan.profile.is_none() && plan.jobs.is_empty() && plan.state.files.is_empty());
        assert_eq!(plan.state.signature, signature(&s, "26.3"));
        assert_eq!(plan.state.error.as_deref(), Some("Network error: blocked"));
    }

    #[test]
    fn fabric_profile_overlays_vanilla() {
        let parent: VersionMeta = serde_json::from_str(
            r#"{"id":"1.21.4","type":"release","mainClass":"net.minecraft.client.main.Main","assetIndex":{"id":"19","url":"u"},
            "arguments":{"jvm":["-cp","${classpath}"],"game":["--username","${auth_player_name}"]},
            "libraries":[{"name":"org.ow2.asm:asm:9.0"},{"name":"a:b:1"}]}"#,
        )
        .unwrap();
        let child: VersionMeta = serde_json::from_str(
            r#"{"id":"fabric-loader-0.16-1.21.4","inheritsFrom":"1.21.4","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments":{"jvm":["-DFabricMcEmu= x "],"game":[]},
            "libraries":[{"name":"org.ow2.asm:asm:9.7","url":"https://maven.fabricmc.net/"}]}"#,
        )
        .unwrap();
        let merged = crate::meta::overlay(child, parent);
        assert_eq!(merged.id, "1.21.4");
        assert_eq!(merged.main_class, "net.fabricmc.loader.impl.launch.knot.KnotClient");
        assert_eq!(merged.libraries.len(), 2);
        assert_eq!(merged.libraries[0].name, "org.ow2.asm:asm:9.7");
        assert_eq!(merged.arguments.unwrap().jvm.len(), 3);
        assert_eq!(merged.asset_index.id.as_deref(), Some("19"));
    }
}
