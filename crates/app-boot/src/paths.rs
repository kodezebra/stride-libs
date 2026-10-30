//! Platform directories, resolved per app name.
//!
//! Each product gets its own namespace from the OS convention, so `pos` and
//! `school` can never collide in `~/.local/share` — which is the failure mode
//! when every app hardcodes the workspace name.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use directories::ProjectDirs;

/// The four directories an app may write to.
pub struct Dirs {
    /// Databases, user-generated files.
    pub data: PathBuf,
    /// Settings files.
    pub config: PathBuf,
    /// Regenerable caches. Safe to delete at any time.
    pub cache: PathBuf,
    /// Log files. Lives under `data` so it survives cache cleaning.
    pub logs: PathBuf,
}

/// Resolve and create the directories for `app`.
pub fn resolve(app: &str) -> Result<Dirs> {
    // Empty qualifier and organisation: on Linux this yields
    // `~/.local/share/<app>`, which is what a Linux-first product wants. Add
    // values here if the apps are ever shipped to macOS/Windows where the
    // reverse-domain form matters.
    let project = ProjectDirs::from("", "", app)
        .with_context(|| format!("no home directory available for {app}"))?;

    let dirs = Dirs {
        data: project.data_dir().to_path_buf(),
        config: project.config_dir().to_path_buf(),
        cache: project.cache_dir().to_path_buf(),
        logs: project.data_dir().join("logs"),
    };

    for directory in [&dirs.data, &dirs.config, &dirs.cache, &dirs.logs] {
        create(directory)?;
    }

    Ok(dirs)
}

fn create(directory: &Path) -> Result<()> {
    std::fs::create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))
}
