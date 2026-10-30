//! Shell state that survives a restart.
//!
//! Kept to what the shell itself owns. App settings are the app's business and
//! live in their own file via the `config` crate.

use std::path::Path;

const FILE: &str = "shell.json";

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct NavState {
    /// Index into the screen list. Validated against the current list on start.
    pub active: usize,
}

impl NavState {
    /// Missing, unreadable, or malformed state falls back to defaults: shell
    /// preferences are a convenience and must never block startup.
    pub fn load(dir: Option<&Path>) -> Self {
        let Some(dir) = dir else {
            return Self::default();
        };
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: Option<&Path>) {
        let Some(dir) = dir else {
            return;
        };
        let Ok(bytes) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let _ = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(dir.join(FILE), bytes));
    }
}
