//! Turns version metadata into concrete files: classpath, natives, asset objects.
use crate::download::Job;
use crate::error::{AppError, Result};
use crate::meta::{AssetIndex, FileRef, Library, VersionMeta};
use crate::paths::GamePaths;
use crate::rules::{rules_allow, Env};
use std::path::{Path, PathBuf};

const DEFAULT_LIBRARY_REPO: &str = "https://libraries.minecraft.net/";
const ASSET_BASE: &str = "https://resources.download.minecraft.net/";

#[derive(Debug, Clone)]
pub struct NativeArchive {
    pub jar: PathBuf,
    pub exclude: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Libraries {
    pub jobs: Vec<Job>,
    pub classpath: Vec<PathBuf>,
    pub natives: Vec<NativeArchive>,
}

/// "group:artifact:version[:classifier]" -> repository-relative jar path.
pub fn maven_path(name: &str) -> Option<String> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let (group, artifact, version) = (parts[0].replace('.', "/"), parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    Some(format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}"))
}

fn library_job(paths: &GamePaths, file: &FileRef, fallback_path: Option<String>, repo: Option<&str>) -> Option<Job> {
    let rel = file.path.clone().or(fallback_path)?;
    let url = if file.url.is_empty() {
        format!("{}{}", repo.unwrap_or(DEFAULT_LIBRARY_REPO), rel)
    } else {
        file.url.clone()
    };
    Some(Job { url, dest: paths.libraries().join(&rel), sha1: file.sha1.clone(), size: file.size })
}

fn resolve_library(lib: &Library, env: &Env, paths: &GamePaths, out: &mut Libraries) {
    if !rules_allow(&lib.rules, env) {
        return;
    }
    let arch_bits = if env.arch == "x86" { "32" } else { "64" };

    match &lib.downloads {
        Some(downloads) => {
            if let Some(artifact) = &downloads.artifact {
                if let Some(job) = library_job(paths, artifact, maven_path(&lib.name), None) {
                    out.classpath.push(job.dest.clone());
                    out.jobs.push(job);
                }
            }
            // Legacy natives: a per-OS classifier jar that is extracted rather than put on the classpath.
            if let (Some(natives), Some(classifiers)) = (&lib.natives, &downloads.classifiers) {
                if let Some(key) = natives.get(env.os).map(|k| k.replace("${arch}", arch_bits)) {
                    if let Some(file) = classifiers.get(&key) {
                        if let Some(job) = library_job(paths, file, None, None) {
                            out.natives.push(NativeArchive {
                                jar: job.dest.clone(),
                                exclude: lib.extract.as_ref().map(|e| e.exclude.clone()).unwrap_or_default(),
                            });
                            out.jobs.push(job);
                        }
                    }
                }
            }
        }
        None => {
            // Old-style library: maven coordinates plus optional repository base URL.
            let file = FileRef::default();
            if let Some(job) = library_job(paths, &file, maven_path(&lib.name), lib.url.as_deref()) {
                out.classpath.push(job.dest.clone());
                out.jobs.push(job);
            }
        }
    }
}

pub fn resolve_libraries(meta: &VersionMeta, env: &Env, paths: &GamePaths) -> Libraries {
    let mut out = Libraries::default();
    for lib in &meta.libraries {
        resolve_library(lib, env, paths, &mut out);
    }
    let mut seen = std::collections::HashSet::new();
    out.classpath.retain(|p| seen.insert(p.clone()));
    out
}

pub fn client_job(meta: &VersionMeta, paths: &GamePaths) -> Result<Job> {
    let client = meta
        .downloads
        .as_ref()
        .and_then(|d| d.client.as_ref())
        .ok_or_else(|| AppError::new("metadata", "This version has no client download."))?;
    Ok(Job { url: client.url.clone(), dest: paths.version_jar(&meta.id), sha1: client.sha1.clone(), size: client.size })
}

pub fn log_config_job(meta: &VersionMeta, paths: &GamePaths) -> Option<Job> {
    let file = &meta.logging.as_ref()?.client.as_ref()?.file;
    let id = file.id.as_deref()?;
    Some(Job { url: file.url.clone(), dest: paths.log_config(id), sha1: file.sha1.clone(), size: file.size })
}

pub fn asset_jobs(index: &AssetIndex, paths: &GamePaths) -> Vec<Job> {
    index
        .objects
        .values()
        .filter(|o| o.hash.len() >= 2)
        .map(|o| Job {
            url: format!("{ASSET_BASE}{}/{}", &o.hash[..2], o.hash),
            dest: paths.asset_object(&o.hash),
            sha1: Some(o.hash.clone()),
            size: Some(o.size),
        })
        .collect()
}

pub fn index_job(meta: &VersionMeta, paths: &GamePaths) -> Job {
    Job {
        url: meta.asset_index.url.clone(),
        dest: paths.asset_index_file(meta.asset_index_id()),
        sha1: meta.asset_index.sha1.clone(),
        size: meta.asset_index.size,
    }
}

/// Extracts legacy native libraries into `dest`, honouring the library's exclude list.
pub fn extract_natives(archives: &[NativeArchive], dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for archive in archives {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&archive.jar)?)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let Some(rel) = entry.enclosed_name() else { continue };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if archive.exclude.iter().any(|ex| rel_str.starts_with(ex.as_str())) {
                continue;
            }
            let target = dest.join(&rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            match std::fs::File::create(&target) {
                Ok(mut out) => {
                    std::io::copy(&mut entry, &mut out)?;
                }
                // A native that is already loaded by a running game cannot be rewritten; it is identical anyway.
                Err(_) if target.exists() => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}

/// Pre-1.7 versions read assets by their logical name rather than by hash.
pub fn materialize_legacy_assets(index: &AssetIndex, index_id: &str, paths: &GamePaths) -> Result<()> {
    let target_root = if index.map_to_resources {
        paths.root.join("resources")
    } else if index.is_virtual {
        paths.virtual_assets(index_id)
    } else {
        return Ok(());
    };
    for (name, object) in &index.objects {
        if object.hash.len() < 2 {
            continue;
        }
        let target = target_root.join(name);
        if std::fs::metadata(&target).is_ok_and(|m| m.len() == object.size) {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(paths.asset_object(&object.hash), &target)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
        assert_eq!(maven_path("net.sf.jopt-simple:jopt-simple:5.0.2").unwrap(), "net/sf/jopt-simple/jopt-simple/5.0.2/jopt-simple-5.0.2.jar");
        assert!(maven_path("broken").is_none());
    }

    fn meta(libs: &str) -> VersionMeta {
        serde_json::from_str(&format!(
            r#"{{"id":"t","type":"release","mainClass":"M","assetIndex":{{"id":"1","url":"u"}},"libraries":{libs}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn resolves_modern_legacy_and_rule_filtered_libraries() {
        let env = Env { os: "windows", arch: "x86_64", os_version: "10.0".into() };
        let paths = GamePaths::new("/g");
        let m = meta(
            r#"[
              {"name":"a:b:1","downloads":{"artifact":{"path":"a/b/1/b-1.jar","url":"http://x/b.jar","sha1":"aa","size":3}}},
              {"name":"a:mac:1","rules":[{"action":"allow","os":{"name":"osx"}}],"downloads":{"artifact":{"path":"a/mac/1/mac-1.jar","url":"http://x/mac.jar"}}},
              {"name":"o:nat:1","natives":{"windows":"natives-windows-${arch}"},"extract":{"exclude":["META-INF/"]},
               "downloads":{"classifiers":{"natives-windows-64":{"path":"o/nat/1/nat-1-natives-windows-64.jar","url":"http://x/n.jar"}}}},
              {"name":"old:lib:2","url":"http://repo/"}
            ]"#,
        );
        let libs = resolve_libraries(&m, &env, &paths);
        assert_eq!(libs.classpath.len(), 2);
        assert_eq!(libs.natives.len(), 1);
        assert_eq!(libs.natives[0].exclude, vec!["META-INF/"]);
        assert_eq!(libs.jobs.len(), 3);
        assert!(libs.jobs.iter().any(|j| j.url == "http://repo/old/lib/2/lib-2.jar"));
    }
}
