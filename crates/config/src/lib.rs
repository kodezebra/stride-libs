//! Settings files.
//!
//! This crate is a *format helper*, not a settings schema: each app declares
//! its own `Settings` struct and decides what belongs in it. Nothing here knows
//! what a "theme" or a "printer" is.
//!
//! JSON rather than TOML so the only dependency is `serde_json`, which is
//! already needed for shell state and is in the local cargo cache.

use std::path::Path;

use anyhow::{Context as _, Result};
use serde::{Serialize, de::DeserializeOwned};

/// Read a settings file. `Ok(None)` means the file does not exist yet, which is
/// an ordinary first run — not an error.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let value = serde_json::from_slice(&bytes)
                .with_context(|| format!("parsing settings {}", path.display()))?;
            Ok(Some(value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading settings {}", path.display())),
    }
}

/// Read a settings file, falling back to `T::default()` on first run.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    Ok(load(path)?.unwrap_or_default())
}

/// Write a settings file via a temporary file plus rename, so a crash or power
/// loss never leaves a half-written file that fails to parse on next start.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value).context("serialising settings")?;
    std::fs::write(&temporary, bytes)
        .with_context(|| format!("writing {}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .with_context(|| format!("moving settings into {}", path.display()))?;
    Ok(())
}
