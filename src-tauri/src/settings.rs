use crate::error::{AppError, Result};
use crate::paths::{self, GamePaths};
use crate::platform;
use crate::system;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LastPlayed {
    pub version: String,
    /// Unix timestamp in seconds.
    pub at: u64,
}

/// Position and visibility of one in-game HUD module. `x`/`y` are fractions of the screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HudModule {
    pub enabled: bool,
    pub x: f32,
    pub y: f32,
}

impl Default for HudModule {
    fn default() -> Self {
        Self { enabled: true, x: 0.0, y: 0.0 }
    }
}

/// Fully customizable in-game overlay, rendered by the bundled `diamond-hud` Fabric mod.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HudSettings {
    pub enabled: bool,
    pub scale: f32,
    pub background_opacity: f32,
    pub accent: String,
    pub text_color: String,
    pub keybind: i32,
    pub corner_radius: i32,
    pub fps: HudModule,
    pub cps: HudModule,
    pub keystrokes: HudModule,
    pub coords: HudModule,
    pub day: HudModule,
    pub ip: HudModule,
    pub armor: HudModule,
    /// Background rendering toggle: makes dark caves and night clearly visible.
    #[serde(default = "default_fullbright_module")]
    pub fullbright: HudModule,
    /// Client-side time of day override.
    #[serde(default = "default_off_module")]
    pub time_changer: HudModule,
    /// Time of day used by the Time Changer module (0..23999 overworld clock ticks).
    #[serde(default = "default_time_changer_time")]
    pub time_changer_time: i32,
    /// Shows the player's current server ping.
    #[serde(default = "default_off_module")]
    pub ping: HudModule,
    /// Shows the measured server tick rate.
    #[serde(default = "default_off_module")]
    pub tps: HudModule,
    /// Shows the direction the player is facing.
    #[serde(default = "default_off_module")]
    pub direction: HudModule,
    /// Toggle sprint behavior (hold vs toggle).
    #[serde(default = "default_off_module")]
    pub toggle_sprint: HudModule,
}

fn default_fullbright_module() -> HudModule {
    HudModule { enabled: false, ..HudModule::default() }
}

fn default_off_module() -> HudModule {
    HudModule { enabled: false, ..HudModule::default() }
}

fn default_time_changer_time() -> i32 {
    6000
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            scale: 1.0,
            background_opacity: 0.55,
            accent: "#6ee7f0".into(),
            text_color: "#e8f0f2".into(),
            keybind: 344,
            corner_radius: 3,
            fps: HudModule { enabled: true, x: 0.015, y: 0.015 },
            cps: HudModule { enabled: true, x: 0.015, y: 0.075 },
            keystrokes: HudModule { enabled: true, x: 0.015, y: 0.150 },
            coords: HudModule { enabled: true, x: 0.015, y: 0.850 },
            day: HudModule { enabled: true, x: 0.015, y: 0.225 },
            ip: HudModule { enabled: true, x: 0.015, y: 0.300 },
            armor: HudModule { enabled: true, x: 0.850, y: 0.015 },
            fullbright: HudModule { enabled: false, x: 0.850, y: 0.075 },
            time_changer: HudModule { enabled: false, x: 0.850, y: 0.135 },
            time_changer_time: 6000,
            ping: HudModule { enabled: false, x: 0.850, y: 0.200 },
            tps: HudModule { enabled: false, x: 0.850, y: 0.330 },
            direction: HudModule { enabled: false, x: 0.850, y: 0.395 },
            toggle_sprint: HudModule { enabled: false, x: 0.850, y: 0.265 },
        }
    }
}

impl HudSettings {
    pub fn active(&self) -> bool {
        self.enabled && (self.fps.enabled || self.cps.enabled || self.keystrokes.enabled || self.coords.enabled || self.day.enabled || self.ip.enabled || self.armor.enabled || self.fullbright.enabled || self.time_changer.enabled || self.ping.enabled || self.tps.enabled || self.direction.enabled || self.toggle_sprint.enabled)
    }

    pub fn normalized(&mut self) {
        self.scale = finite(self.scale, 1.0).clamp(0.5, 2.0);
        self.background_opacity = finite(self.background_opacity, 0.55).clamp(0.0, 1.0);
        self.accent = normalize_color(&self.accent, "#6ee7f0");
        self.text_color = normalize_color(&self.text_color, "#e8f0f2");
        self.keybind = self.keybind.clamp(0, 1000);
        self.corner_radius = self.corner_radius.clamp(0, 3);
        self.time_changer_time = self.time_changer_time.clamp(0, 23999);
        for module in [&mut self.fps, &mut self.cps, &mut self.keystrokes, &mut self.coords, &mut self.day, &mut self.ip, &mut self.armor, &mut self.fullbright, &mut self.time_changer, &mut self.ping, &mut self.tps, &mut self.direction, &mut self.toggle_sprint] {
            module.x = finite(module.x, 0.0).clamp(0.0, 1.0);
            module.y = finite(module.y, 0.0).clamp(0.0, 1.0);
        }
    }

    pub fn into_normalized(mut self) -> Self {
        self.normalized();
        self
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn normalize_color(value: &str, fallback: &str) -> String {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("#{}", hex.to_ascii_lowercase())
    } else {
        fallback.to_string()
    }
}

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
    pub disabled_mods: Vec<String>,
    pub hud: HudSettings,
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
            show_snapshots: true,
            mods_enabled: true,
            disabled_mods: Vec::new(),
            hud: HudSettings::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let file = paths::settings_file();
        if file.exists() {
            if let Ok(bytes) = std::fs::read(&file) {
                if let Ok(mut settings) = serde_json::from_slice::<Settings>(&bytes) {
                    settings.normalize(0);
                    return settings;
                }
            }
        }
        Self::default()
    }

    fn normalize(&mut self, _total_ram_mb: u64) {
        if self.username.is_empty() {
            self.username = "Player".into();
        }
        if self.jvm_args.is_empty() {
            self.jvm_args = String::new();
        }
        if self.username.len() > 16 {
            self.username.truncate(16);
        }
        self.hud.normalized();
    }

    pub fn normalized(mut self, _total_ram_mb: u64) -> crate::error::Result<Self> {
        if self.username.is_empty() {
            self.username = "Player".into();
        }
        if self.jvm_args.is_empty() {
            self.jvm_args = String::new();
        }
        if self.username.len() > 16 {
            self.username.truncate(16);
        }
        self.max_ram_mb = self.max_ram_mb.or_else(|| {
            let (total, available) = system::memory_mb();
            Some(system::recommended_ram_mb(total, available, platform::arch()))
        });
        self.hud.normalized();
        Ok(self)
    }

    pub fn save(&self) -> Result<()> {
        let file = paths::settings_file();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(file, json)?;
        Ok(())
    }

    pub fn game_paths(&self) -> GamePaths {
        let dir = self.game_dir.clone().unwrap_or_else(|| paths::default_game_dir().to_string_lossy().into_owned());
        GamePaths::new(dir)
    }
}

pub fn validate_username(name: &str) -> crate::error::Result<()> {
    if name.is_empty() {
        return Err(AppError::new("username", "Username cannot be empty."));
    }
    if name.len() > 16 {
        return Err(AppError::new("username", "Username cannot be longer than 16 characters."));
    }
    if name.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
        return Err(AppError::new("username", "Username may only contain ASCII letters, digits and underscores."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_defaults_are_sane() {
        let s = Settings::default();
        assert_eq!(s.username, "Player");
        assert!(s.mods_enabled);
        assert!(s.hud.enabled);
    }

    #[test]
    fn hud_settings_active_reflects_modules() {
        let mut h = HudSettings::default();
        assert!(h.active());
        h.fps.enabled = false;
        h.cps.enabled = false;
        h.keystrokes.enabled = false;
        h.coords.enabled = false;
        assert!(h.active());
        h.day.enabled = false;
        h.ip.enabled = false;
        h.armor.enabled = false;
        assert!(!h.active());
    }

    #[test]
    fn time_changer_time_is_clamped() {
        let mut h = HudSettings::default();
        assert_eq!(h.time_changer_time, 6000);
        h.time_changer_time = 99999;
        h.normalized();
        assert_eq!(h.time_changer_time, 23999);
        h.time_changer_time = -5;
        h.normalized();
        assert_eq!(h.time_changer_time, 0);
    }
}