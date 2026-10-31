//! Process plumbing every stride app needs before it can do anything useful.
//!
//! `app-boot` answers "how does the process behave": where its files live, where
//! logs go, what happens on a panic, and what exit code the shell sees. It
//! answers nothing about the product — it must never depend on `ui-kit`,
//! `storage`, or an app, or it becomes the `core` crate this layout exists to
//! avoid.
//!
//! The acceptance test for this crate is simple: **an item belongs here only if
//! every app calls it from the first line of `main`.**

mod crash;
mod logging;
mod paths;

use std::path::PathBuf;
use std::process::ExitCode;

pub use paths::Dirs;

/// Everything `init` resolved, passed to the app's body.
pub struct Context {
    /// Product name as it appears in paths and logs, e.g. `"pos"`.
    pub name: &'static str,
    pub dirs: Dirs,
}

impl Context {
    /// Path inside the app's data directory, e.g. `"pos.db"`.
    pub fn data_path(&self, file: &str) -> PathBuf {
        self.dirs.data.join(file)
    }

    /// Path inside the app's config directory, e.g. `"settings.json"`.
    pub fn config_path(&self, file: &str) -> PathBuf {
        self.dirs.config.join(file)
    }
}

/// Install logging and the panic hook, then resolve and create directories.
pub fn init(name: &'static str) -> anyhow::Result<Context> {
    let dirs = paths::resolve(name)?;
    logging::init(name, &dirs.logs)?;
    crash::install(name);
    tracing::info!(
        app = name,
        data = %dirs.data.display(),
        config = %dirs.config.display(),
        "starting"
    );
    Ok(Context { name, dirs })
}

/// Standard entry point: `init`, run the app body, report failures.
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     app_boot::run("pos", |ctx| {
///         println!("data lives in {}", ctx.dirs.data.display());
///         Ok(())
///     })
/// }
/// ```
pub fn run<F>(name: &'static str, body: F) -> ExitCode
where
    F: FnOnce(&Context) -> anyhow::Result<()>,
{
    let context = match init(name) {
        Ok(context) => context,
        Err(error) => {
            // Logging may be exactly what failed, so write to stderr directly.
            eprintln!("{name}: could not start: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    match body(&context) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error:#}");
            eprintln!("{name}: {error:#}");
            ExitCode::FAILURE
        }
    }
}
