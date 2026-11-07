use std::sync::Mutex;
use tokio::sync::oneshot;

pub struct GameHandle {
    pub version: String,
    pub stop: Option<oneshot::Sender<()>>,
}

pub struct AppState {
    pub http: reqwest::Client,
    pub installing: Mutex<Option<String>>,
    pub game: Mutex<Option<GameHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("DiamondLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self { http, installing: Mutex::new(None), game: Mutex::new(None) }
    }
}

/// Marks an install as running for as long as it is alive.
pub struct InstallGuard<'a>(&'a AppState);

impl<'a> InstallGuard<'a> {
    pub fn acquire(state: &'a AppState, version: &str) -> Option<Self> {
        let mut slot = state.installing.lock().ok()?;
        if slot.is_some() {
            return None;
        }
        *slot = Some(version.to_string());
        Some(Self(state))
    }
}

impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.0.installing.lock() {
            *slot = None;
        }
    }
}
