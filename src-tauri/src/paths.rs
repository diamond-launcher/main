use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

/// Launcher-private data (settings, manifest cache). Not the game directory.
pub fn data_root() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Diamond")
}

pub fn settings_file() -> PathBuf {
    data_root().join("settings.json")
}

pub fn manifest_cache_file() -> PathBuf {
    data_root().join("version_manifest.json")
}

pub fn default_game_dir() -> PathBuf {
    data_root().join("minecraft")
}

/// Version ids come from remote metadata and become path segments; reject anything suspicious.
pub fn validate_version_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ' | '+'));
    if ok {
        Ok(())
    } else {
        Err(AppError::new("invalid_version", format!("Invalid version id: {id}")))
    }
}

/// Standard `.minecraft`-style directory layout rooted at the game directory.
#[derive(Debug, Clone)]
pub struct GamePaths {
    pub root: PathBuf,
}

impl GamePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.versions().join(id)
    }
    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.json"))
    }
    pub fn version_jar(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.jar"))
    }
    pub fn natives(&self, id: &str) -> PathBuf {
        self.version_dir(id).join("natives")
    }
    pub fn installed_marker(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(".diamond-installed")
    }
    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn asset_index_file(&self, index_id: &str) -> PathBuf {
        self.assets().join("indexes").join(format!("{index_id}.json"))
    }
    pub fn asset_object(&self, hash: &str) -> PathBuf {
        self.assets().join("objects").join(&hash[..2]).join(hash)
    }
    pub fn virtual_assets(&self, index_id: &str) -> PathBuf {
        self.assets().join("virtual").join(index_id)
    }
    pub fn log_config(&self, file_id: &str) -> PathBuf {
        self.assets().join("log_configs").join(file_id)
    }
    /// Launcher-managed mods for one Minecraft version (kept apart from the user's own `mods` folder).
    pub fn mods_dir(&self, mc: &str) -> PathBuf {
        self.root.join("diamond-mods").join(mc)
    }
    pub fn mods_state_file(&self, mc: &str) -> PathBuf {
        self.root.join("diamond-mods").join(format!("{mc}.state.json"))
    }
    pub fn runtime_dir(&self, component: &str) -> PathBuf {
        self.root.join("runtime").join(component)
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_ids() {
        assert!(validate_version_id("1.20.4").is_ok());
        assert!(validate_version_id("24w14a").is_ok());
        assert!(validate_version_id("../evil").is_err());
        assert!(validate_version_id("a/b").is_err());
        assert!(validate_version_id("").is_err());
    }

    #[test]
    fn layout_matches_minecraft_convention() {
        let p = GamePaths::new("/g");
        assert!(p.version_json("1.0").ends_with("versions/1.0/1.0.json") || p.version_json("1.0").ends_with("versions\\1.0\\1.0.json"));
        assert!(p.asset_object("abcdef").to_string_lossy().contains("objects"));
    }
}
