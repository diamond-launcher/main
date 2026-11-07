//! Official version manifest retrieval and per-version JSON handling.
use crate::download::{self, Job, Progress};
use crate::error::{AppError, Result};
use crate::meta::VersionMeta;
use crate::paths::{self, GamePaths};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: String,
    pub release_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

/// Fetches the manifest, caching it on disk. Falls back to the cache when offline.
/// The boolean is `true` when the cached copy was used.
pub async fn load(client: &reqwest::Client) -> Result<(Manifest, bool)> {
    let cache = paths::manifest_cache_file();
    let fetched = async {
        let bytes = client.get(MANIFEST_URL).send().await?.error_for_status()?.bytes().await?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        Ok::<_, AppError>((manifest, bytes))
    }
    .await;

    match fetched {
        Ok((manifest, bytes)) => {
            if let Some(dir) = cache.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&cache, &bytes);
            Ok((manifest, false))
        }
        Err(err) => match std::fs::read(&cache).ok().and_then(|b| serde_json::from_slice::<Manifest>(&b).ok()) {
            Some(manifest) => Ok((manifest, true)),
            None => Err(err),
        },
    }
}

pub fn read_local_meta(paths: &GamePaths, id: &str) -> Result<VersionMeta> {
    let file = paths.version_json(id);
    let bytes = std::fs::read(&file)
        .map_err(|_| AppError::new("not_installed", format!("Version {id} is not installed.")))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Returns the version JSON, downloading or refreshing it from Mojang when needed.
pub async fn ensure_meta(client: &reqwest::Client, paths: &GamePaths, id: &str) -> Result<VersionMeta> {
    paths::validate_version_id(id)?;
    let local_file = paths.version_json(id);
    let local_sha1 = download::sha1_file(&local_file).ok();

    let entry = match load(client).await {
        Ok((manifest, _)) => manifest.versions.into_iter().find(|v| v.id == id),
        Err(_) if local_sha1.is_some() => None,
        Err(e) => return Err(e),
    };

    match entry {
        Some(entry) => {
            let up_to_date = local_sha1.as_deref().is_some_and(|h| h.eq_ignore_ascii_case(&entry.sha1));
            if !up_to_date {
                let job = Job { url: entry.url, dest: local_file, sha1: Some(entry.sha1).filter(|s| !s.is_empty()), size: None };
                download::download_all(client, vec![job], Arc::new(Progress::default())).await?;
            }
        }
        None if local_sha1.is_none() => {
            return Err(AppError::new("unknown_version", format!("Minecraft version {id} was not found.")));
        }
        None => {}
    }

    let meta = read_local_meta(paths, id)?;
    if meta.inherits_from.is_some() {
        return Err(AppError::new("unsupported", "Versions that inherit from another version are not supported."));
    }
    Ok(meta)
}
