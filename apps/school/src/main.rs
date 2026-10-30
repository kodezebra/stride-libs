//! Zebra School.
//!
//! Same crate wiring as `pos` and a completely different product: the shared
//! crates supply chrome, storage, settings, and process behaviour, and this app
//! supplies its own screens and schema.

mod db;
mod screens;

use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use ui_shell::Shell;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    /// "light" or "dark". Unknown values fall back to the default theme.
    theme: Option<String>,
}

fn main() -> ExitCode {
    app_boot::run("school", |ctx| {
        let mut store = storage::Store::open(ctx.data_path("school.db"))?;
        // Claim before migrating: a file that belongs to another app is refused
        // before this one writes a single table into it.
        store.claim_application_id(db::APPLICATION_ID)?;
        store.migrate(db::MIGRATIONS)?;

        let settings_path = ctx.config_path("settings.json");
        let settings: Settings = config::load_or_default(&settings_path)?;
        let theme = settings
            .theme
            .as_deref()
            .map(ui_kit::theme::Theme::parse)
            .unwrap_or_default();

        let state = screens::State::new(store);

        Shell::new("Zebra School")
            .theme(theme)
            .persist_in(ctx.dirs.config.clone())
            .screen(screens::StudentsScreen)
            .screen(screens::ClassesScreen)
            .header_action("Enrol student", |state| state.enrolled += 1)
            .status_bar(|state| format!("{} student(s) enrolled", state.enrolled))
            .run(state)?;

        Ok(())
    })
}
