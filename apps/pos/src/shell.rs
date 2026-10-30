//! The POS frame.
//!
//! Sell is not a page inside the school layout with different labels; it is a
//! different arrangement: no header, a 52 px activity rail, a sidebar that
//! belongs to the module, and a cart pinned to the right of the workspace. That
//! is why this app composes [`ui_shell::regions`] instead of using
//! [`ui_shell::Shell`] — same chrome, different skeleton.
//!
//! Keyboard first, because a till is operated without looking at a menu:
//!
//! | Key         | Action                    |
//! |-------------|---------------------------|
//! | `F1`..`F5`  | Switch module             |
//! | `F6`..`F8`  | Reserved: inventory, suppliers, purchase orders |
//! | `Ctrl+B`    | Collapse/expand sidebar   |
//! | `Ctrl+P`    | Command palette           |

use std::path::PathBuf;

use ui_kit::components;
use ui_kit::icons::Icon;
use ui_kit::theme::Theme;
use ui_kit::tokens::TYPE;
use ui_shell::egui;
use ui_shell::regions::{self, Command, Palette, RailItem, Sidebar, size};

use crate::modules::{self, Module};
use crate::prefs::Prefs;
use crate::state::{self, State};

pub struct PosShell {
    state: State,
    modules: Vec<Box<dyn Module>>,
    /// Built once from `modules` so the rail and the tooltips cannot drift from
    /// the hotkey router: both read the same `Module` methods.
    rail: Vec<RailItem>,
    commands: Vec<Command<State>>,
    palette: Palette,
    prefs_path: PathBuf,
}

impl PosShell {
    pub fn new(state: State, prefs_path: PathBuf) -> Self {
        let modules = modules::all();
        let rail = modules
            .iter()
            .map(|module| RailItem {
                icon: module.icon(),
                label: module.title(),
                hotkey: module.hotkey().name(),
            })
            .collect();

        Self {
            state,
            modules,
            rail,
            commands: crate::commands::all(),
            palette: Palette::default(),
            prefs_path,
        }
    }

    /// Entry point, handed to [`ui_shell::regions::run_app`].
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        // A theme switch takes effect here, at the top of the next frame. The
        // shell is the only place that installs global visuals; the Settings
        // screen only records the intent on `state`.
        if self.state.theme_dirty {
            ui_kit::theme::install(ui.ctx(), self.state.theme);
            self.state.theme_dirty = false;
        }

        self.handle_keys(ui.ctx());
        self.draw_frame(ui);
        // Drawn after the frame so the modal's backdrop covers it. `Modal` puts
        // itself in the foreground layer, but a same-frame repaint is cheaper
        // than waiting for one more.
        self.palette.show(ui.ctx(), &self.commands, &mut self.state);
        self.persist();
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let mut jump = None;
        ctx.input(|input| {
            for (index, module) in self.modules.iter().enumerate() {
                if input.key_pressed(module.hotkey()) {
                    jump = Some(index);
                }
            }
        });
        if let Some(index) = jump {
            self.state.select(index);
        }

        let (toggle_sidebar, open_palette, toggle_theme) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                command && input.key_pressed(egui::Key::B),
                command && input.key_pressed(egui::Key::P),
                command && input.key_pressed(egui::Key::T),
            )
        });

        if toggle_sidebar {
            self.state.toggle_sidebar();
        }
        if open_palette {
            self.palette.request_open();
        }
        if toggle_theme {
            self.state.toggle_theme();
        }
    }

    fn draw_frame(&mut self, ui: &mut egui::Ui) {
        // Borrow the fields separately: the closures below need `state` mutably
        // while the frame reads `modules` and `rail`.
        let Self {
            state,
            modules,
            rail,
            ..
        } = self;

        let active = state.module;

        regions::status(ui, |ui| status_bar(ui, state));

        if let Some(next) = regions::rail(ui, rail, active) {
            state.select(next);
        }

        let module = &mut modules[active];
        let spec = match module.fixed_sidebar() {
            Some(width) => Sidebar::new(width).fixed(),
            None => Sidebar::default(),
        };

        // `show_collapsible` flips this flag when the user drags the sidebar
        // shut, so the state is committed after the frame rather than read once.
        let mut open = state.sidebar_open(active);
        regions::sidebar(ui, spec, &mut open, |ui| module.sidebar(ui, state));
        state.set_sidebar_open(active, open);

        regions::workspace(ui, |ui| modules[active].workspace(ui, state));
    }

    fn persist(&mut self) {
        if self.state.prefs_dirty {
            let prefs = Prefs {
                module: self.state.module,
                sidebar: self.state.sidebar_open.clone(),
            };

            match config::save(&self.prefs_path, &prefs) {
                Ok(()) => self.state.prefs_dirty = false,
                Err(err) => {
                    // Clear the flag anyway: retrying every frame would rewrite
                    // the log 60 times a second, and a config directory that
                    // cannot be written is not something this loop can fix.
                    self.state.prefs_dirty = false;
                    tracing::warn!(error = %err, path = %self.prefs_path.display(), "could not save prefs");
                }
            }
        }

        if self.state.settings_dirty {
            let path = self.state.settings_path.clone();
            match config::save(&path, &self.state.settings) {
                Ok(()) => self.state.settings_dirty = false,
                Err(err) => {
                    self.state.settings_dirty = false;
                    tracing::warn!(error = %err, path = %path.display(), "could not save settings");
                }
            }
        }
    }
}

fn status_bar(ui: &mut egui::Ui, state: &mut State) {
    let muted = ui.visuals().weak_text_color();

    // Left cluster: what the terminal knows about itself. "local only" is the
    // truth today — there is no hub and no cloud — and it is what the operator
    // needs to see before they trust the queue depth that will sit next to it.
    regions::status_item(ui, Icon::UNSYNCED, "local only", muted);

    let cart = format!(
        "{} item(s) - {}",
        state.cart_count(),
        state::money(state.cart_total_cents())
    );

    // Shortcut hints live where the eye already is: the bar is always visible,
    // so the keys are learned once instead of being buried in a help screen.
    hint(ui, "Ctrl+P", "commands", muted);
    hint(ui, "Ctrl+B", "sidebar", muted);

    regions::status_end(ui, |ui| {
        // Right-to-left layout: register in reverse of the visual order.
        ui.label(
            egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                .size(TYPE.status)
                .weak(),
        );
        theme_toggle(ui, state);
        regions::status_item(ui, Icon::USER, "no cashier", muted);
        regions::status_item(ui, Icon::CART, &cart, muted);
    });
}

/// A key and what it does, in one muted segment.
fn hint(ui: &mut egui::Ui, key: &str, action: &str, muted: egui::Color32) {
    ui.horizontal_centered(|ui| {
        ui.label(
            egui::RichText::new(key)
                .size(TYPE.status)
                .monospace()
                .color(ui.visuals().strong_text_color()),
        );
        ui.label(egui::RichText::new(action).size(TYPE.status).color(muted));
    });
}

/// Appearance toggle. Settings still exists for the people who look there, but
/// flipping the lights should not cost a screen change.
fn theme_toggle(ui: &mut egui::Ui, state: &mut State) {
    let dark = state.theme == Theme::Dark;
    let icon = if dark { Icon::SUN } else { Icon::MOON };
    let tip = if dark {
        "Switch to light (Ctrl+T)"
    } else {
        "Switch to dark (Ctrl+T)"
    };

    if components::icon_button(ui, icon, size::STATUS_ICON + 4.0)
        .on_hover_text(tip)
        .clicked()
    {
        state.toggle_theme();
    }
}
