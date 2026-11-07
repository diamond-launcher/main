/// Operating system name using Mojang's rule vocabulary.
pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// CPU architecture using Mojang's rule vocabulary.
pub fn arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    }
}

pub fn classpath_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Platform key used by Mojang's Java runtime index.
pub fn runtime_key() -> Option<&'static str> {
    match (os_name(), arch()) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("windows", "x86") => Some("windows-x86"),
        ("windows", "arm64") => Some("windows-arm64"),
        ("osx", "x86_64") => Some("mac-os"),
        ("osx", "arm64") => Some("mac-os-arm64"),
        ("linux", "x86_64") => Some("linux"),
        ("linux", "x86") => Some("linux-i386"),
        _ => None,
    }
}

/// Prevents a console window from flashing when spawning child processes on Windows.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
