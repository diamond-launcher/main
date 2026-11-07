use crate::error::{AppError, Result};
use crate::paths::{self, GamePaths};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LastPlayed {
    pub version: String,
    /// Unix timestamp in seconds.
    pub at: u64,
}

/// Every optional field means "detect automatically" when `None`/empty.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub username: String,
    pub max_ram_mb: Option<u32>,
    pub java_path: Option<String>,
    pub game_dir: Option<String>,
    pub jvm_args: String,
    pub selected_version: Option<String>,
    pub last_played: Option<LastPlayed>,
    pub show_snapshots: bool,
    pub mods_enabled: bool,
    /// Catalog slugs the user switched off.
    pub disabled_mods: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            username: "Player".into(),
            max_ram_mb: None,
            java_path: None,
            game_dir: None,
            jvm_args: String::new(),
            selected_version: None,
            last_played: None,
            show_snapshots: false,
            mods_enabled: true,
            disabled_mods: Vec::new(),
        }
    }
}

pub const MIN_RAM_MB: u32 = 512;

pub fn validate_username(name: &str) -> Result<()> {
    let ok = (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok {
        Ok(())
    } else {
        Err(AppError::new("invalid_username", "Username must be 3–16 characters: letters, numbers or underscore."))
    }
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read(paths::settings_file())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let file = paths::settings_file();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &file)?;
        Ok(())
    }

    pub fn game_root(&self) -> PathBuf {
        match self.game_dir.as_deref().map(str::trim) {
            Some(d) if !d.is_empty() => PathBuf::from(d),
            _ => paths::default_game_dir(),
        }
    }

    pub fn game_paths(&self) -> GamePaths {
        GamePaths::new(self.game_root())
    }

    /// Trims, drops empty optionals and validates user-provided values.
    pub fn normalized(mut self, total_ram_mb: u64) -> Result<Self> {
        self.username = self.username.trim().to_string();
        validate_username(&self.username)?;
        self.java_path = self.java_path.take().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.game_dir = self.game_dir.take().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.jvm_args = self.jvm_args.trim().to_string();
        self.disabled_mods.retain(|s| crate::mods::CATALOG.iter().any(|c| c.slug == s));
        self.disabled_mods.sort();
        self.disabled_mods.dedup();
        if let Some(ram) = self.max_ram_mb {
            if ram < MIN_RAM_MB || u64::from(ram) > total_ram_mb {
                return Err(AppError::new(
                    "invalid_ram",
                    format!("Memory must be between {MIN_RAM_MB} MB and {total_ram_mb} MB."),
                ));
            }
        }
        if let Some(p) = &self.java_path {
            if !std::path::Path::new(p).is_file() {
                return Err(AppError::new("invalid_java", "The Java executable path does not point to a file."));
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rules() {
        assert!(validate_username("Steve").is_ok());
        assert!(validate_username("ab").is_err());
        assert!(validate_username("has space").is_err());
        assert!(validate_username("this_name_is_way_too_long").is_err());
    }

    #[test]
    fn normalization_clears_blank_values_and_checks_ram() {
        let s = Settings { java_path: Some("  ".into()), max_ram_mb: Some(2048), ..Settings::default() };
        let n = s.normalized(8192).unwrap();
        assert!(n.java_path.is_none());
        let bad = Settings { max_ram_mb: Some(64), ..Settings::default() };
        assert_eq!(bad.normalized(8192).unwrap_err().code, "invalid_ram");
    }

    #[test]
    fn partial_json_falls_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"username":"Alex"}"#).unwrap();
        assert_eq!(s.username, "Alex");
        assert!(!s.show_snapshots);
    }
}
