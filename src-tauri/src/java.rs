//! Java detection and Mojang-managed Java runtimes.
use crate::download::Job;
use crate::error::{AppError, Result};
use crate::meta::{FileRef, VersionMeta};
use crate::paths::{path_str, GamePaths};
use crate::platform;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RUNTIME_INDEX_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    pub path: String,
    pub major: u32,
}

/// Extracts the major version from `java -version` output ("1.8.0_x" -> 8, "17.0.9" -> 17).
pub fn parse_major(output: &str) -> Option<u32> {
    let re = Regex::new(r#"version "(\d+)(?:\.(\d+))?"#).ok()?;
    let caps = re.captures(output)?;
    let first: u32 = caps[1].parse().ok()?;
    if first == 1 {
        caps.get(2)?.as_str().parse().ok()
    } else {
        Some(first)
    }
}

pub fn probe(path: &Path) -> Option<JavaInstall> {
    let mut cmd = Command::new(path);
    cmd.arg("-version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(platform::CREATE_NO_WINDOW);
    }
    let output = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    Some(JavaInstall { path: path_str(path), major: parse_major(&text)? })
}

fn candidate_executables() -> Vec<PathBuf> {
    let exe = platform::exe_name("java");
    let mut found = Vec::new();
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        found.push(PathBuf::from(home).join("bin").join(&exe));
    }
    if let Some(path) = std::env::var_os("PATH") {
        found.extend(std::env::split_paths(&path).map(|p| p.join(&exe)));
    }
    if cfg!(windows) {
        let vendors = ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "BellSoft", "Amazon Corretto", "Semeru"];
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Some(base) = std::env::var_os(var) else { continue };
            for vendor in vendors {
                let Ok(entries) = std::fs::read_dir(PathBuf::from(&base).join(vendor)) else { continue };
                found.extend(entries.flatten().map(|e| e.path().join("bin").join(&exe)));
            }
        }
    } else {
        for base in ["/usr/lib/jvm", "/Library/Java/JavaVirtualMachines"] {
            let Ok(entries) = std::fs::read_dir(base) else { continue };
            for e in entries.flatten() {
                found.push(e.path().join("bin").join(&exe));
                found.push(e.path().join("Contents/Home/bin").join(&exe));
            }
        }
    }
    found.retain(|p| p.is_file());
    found.dedup();
    found
}

/// All Java installations discovered on the system, newest first.
pub fn find_system_javas() -> Vec<JavaInstall> {
    let mut seen = std::collections::HashSet::new();
    let mut installs: Vec<JavaInstall> = candidate_executables()
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .filter_map(|p| probe(&p))
        .collect();
    installs.sort_by(|a, b| b.major.cmp(&a.major));
    installs
}

pub fn managed_java(paths: &GamePaths, component: &str) -> Option<PathBuf> {
    let base = paths.runtime_dir(component);
    let exe = platform::exe_name("java");
    [base.join("bin").join(&exe), base.join("jre.bundle/Contents/Home/bin").join(&exe)]
        .into_iter()
        .find(|p| p.is_file())
}

/// Picks the Java to launch with: user override, then Mojang's managed runtime, then a system install.
pub fn resolve(custom: Option<&str>, meta: &VersionMeta, paths: &GamePaths) -> Result<JavaInstall> {
    let required = meta.required_java_major();
    if let Some(custom) = custom {
        let install = probe(Path::new(custom))
            .ok_or_else(|| AppError::new("invalid_java", format!("Could not run the configured Java at {custom}.")))?;
        if install.major < required {
            return Err(AppError::new(
                "java_too_old",
                format!("This version needs Java {required}, but the configured Java is version {}.", install.major),
            ));
        }
        return Ok(install);
    }
    if let Some(path) = managed_java(paths, meta.java_component()) {
        return Ok(JavaInstall { path: path_str(&path), major: required });
    }
    find_system_javas()
        .into_iter()
        .find(|j| j.major >= required)
        .ok_or_else(|| AppError::new("java_missing", format!("Java {required} is required but was not found.")))
}

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: FileRef,
}

#[derive(Deserialize)]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    downloads: Option<RuntimeDownloads>,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Deserialize)]
struct RuntimeDownloads {
    raw: FileRef,
}

#[derive(Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Default)]
pub struct RuntimePlan {
    pub jobs: Vec<Job>,
    pub executables: Vec<PathBuf>,
    pub links: Vec<(PathBuf, String)>,
}

/// Fetches Mojang's runtime manifest for `component`. `None` if this platform has no managed runtime.
pub async fn runtime_plan(client: &reqwest::Client, paths: &GamePaths, component: &str) -> Result<Option<RuntimePlan>> {
    let Some(key) = platform::runtime_key() else { return Ok(None) };
    let index: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> =
        client.get(RUNTIME_INDEX_URL).send().await?.error_for_status()?.json().await?;
    let Some(entry) = index.get(key).and_then(|c| c.get(component)).and_then(|v| v.first()) else {
        return Ok(None);
    };
    let manifest: RuntimeManifest =
        client.get(&entry.manifest.url).send().await?.error_for_status()?.json().await?;

    let root = paths.runtime_dir(component);
    let mut plan = RuntimePlan::default();
    for (rel, file) in manifest.files {
        let dest = root.join(&rel);
        match file.kind.as_str() {
            "file" => {
                if let Some(d) = file.downloads {
                    if file.executable {
                        plan.executables.push(dest.clone());
                    }
                    plan.jobs.push(Job { url: d.raw.url, dest, sha1: d.raw.sha1, size: d.raw.size });
                }
            }
            "link" => {
                if let Some(target) = file.target {
                    plan.links.push((dest, target));
                }
            }
            _ => {}
        }
    }
    Ok(Some(plan))
}

/// Applies executable bits and symlinks from the runtime manifest (no-op on Windows).
pub fn finalize_runtime(plan: &RuntimePlan) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for exe in &plan.executables {
            if exe.exists() {
                std::fs::set_permissions(exe, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        for (link, target) in &plan.links {
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _ = std::fs::remove_file(link);
            std::os::unix::fs::symlink(target, link)?;
        }
    }
    #[cfg(not(unix))]
    let _ = plan;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_java_versions() {
        assert_eq!(parse_major("openjdk version \"17.0.9\" 2023-10-17"), Some(17));
        assert_eq!(parse_major("java version \"1.8.0_401\""), Some(8));
        assert_eq!(parse_major("openjdk version \"21\" 2023-09-19"), Some(21));
        assert_eq!(parse_major("openjdk version \"25.0.5\" 2026-01-20"), Some(25));
        assert_eq!(parse_major("garbage"), None);
    }
}
