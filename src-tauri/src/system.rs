//! Hardware / platform detection and automatic defaults.
use crate::java::{self, JavaInstall};
use crate::paths;
use crate::platform;
use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: &'static str,
    pub arch: &'static str,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub recommended_ram_mb: u32,
    pub default_game_dir: String,
    pub system_java: Option<JavaInstall>,
}

pub fn memory_mb() -> (u64, u64) {
    let mut sys = System::new();
    sys.refresh_memory();
    (sys.total_memory() / (1024 * 1024), sys.available_memory() / (1024 * 1024))
}

/// Conservative allocation: 40% of installed RAM, never starving the OS, 1–8 GB, in 256 MB steps.
pub fn recommended_ram_mb(total_mb: u64, available_mb: u64, arch: &str) -> u32 {
    let by_total = total_mb * 40 / 100;
    let by_available = available_mb.saturating_sub(1536).max(1024);
    let mut value = by_total.min(by_available).min(8192);
    if arch == "x86" {
        value = value.min(1024);
    }
    ((value / 256) * 256).max(1024) as u32
}

pub fn detect() -> SystemInfo {
    let (total, available) = memory_mb();
    SystemInfo {
        os: platform::os_name(),
        arch: platform::arch(),
        total_ram_mb: total,
        available_ram_mb: available,
        recommended_ram_mb: recommended_ram_mb(total, available, platform::arch()),
        default_game_dir: paths::path_str(&paths::default_game_dir()),
        system_java: java::find_system_javas().into_iter().next(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_recommendation_is_conservative() {
        assert_eq!(recommended_ram_mb(32768, 20000, "x86_64"), 8192);
        assert_eq!(recommended_ram_mb(8192, 5000, "x86_64"), 3072);
        assert_eq!(recommended_ram_mb(4096, 2000, "x86_64"), 1024);
        assert_eq!(recommended_ram_mb(16384, 12000, "x86_64"), 6400);
        assert_eq!(recommended_ram_mb(32768, 30000, "x86"), 1024);
    }
}
