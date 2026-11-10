//! Shared window chrome for every stride product.
//!
//! An app declares its screens and slots; this crate draws the header, the
//! navigation sidebar, the status bar, and the content frame around whichever
//! screen is active.
//!
//! Two rules keep the shared layout from becoming a cage:
//!
//! 1. **Slots, not trait growth.** When one product needs an extra header
//!    button, pass it to [`Shell::header_action`] — never add a method to
//!    [`Screen`] for it, because every other product then pays for it.
//! 2. **`ui-shell` is opt-in.** `ui-kit` is the floor. A product that genuinely
//!    needs different chrome (kiosk, full-screen dashboard) builds its layout
//!    from `ui-kit` primitives and skips this crate entirely.

mod nav;
pub mod regions;

use std::error::Error;
use std::path::PathBuf;

use ui_kit::theme::Theme;
use ui_kit::tokens::{Palette, Rgb};

/// Re-exported so apps never declare their own egui dependency and can never
/// drift onto a second, incompatible egui version.
pub use egui;

/// The rendering backend each platform gets.
///
/// Linux keeps glow: Mesa's llvmpipe provides OpenGL 2.0 even in a VM with no
/// GPU, and no Vulkan driver is required. Windows has no OpenGL in the OS — a
/// PC without a vendor driver offers only Microsoft's software OpenGL 1.1, and
/// glow refuses to start with `egui_glow requires opengl 2.0+`. wgpu draws with
/// Direct3D 12, whose software adapter (WARP) ships with Windows 10 and later,
/// so the window still opens on a machine with no graphics driver at all.
#[cfg(windows)]
const RENDERER: eframe::Renderer = eframe::Renderer::Wgpu;
#[cfg(not(windows))]
const RENDERER: eframe::Renderer = eframe::Renderer::Glow;

/// Window options both entry points share, so the backend is decided here once
/// instead of being repeated (and eventually diverging) in each runner.
fn native_options(viewport: egui::ViewportBuilder) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport,
        renderer: RENDERER,
        ..Default::default()
    }
}

/// One paginated area of a product. Implemented by the app, never by a crate.
pub trait Screen<S> {
    /// Sidebar label and the identifier used in logs.
    fn title(&self) -> &str;

    /// Optional leading glyph for the sidebar entry, e.g. `Some("🛒")`.
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Draw the screen's content. The shell owns the surrounding frame; a
    /// screen owns everything inside it, including its own scroll areas.
    fn draw(&mut self, ui: &mut egui::Ui, state: &mut S);
}

/// A button in the header, bound to one product action.
pub struct HeaderAction<S> {
    label: String,
    on_click: Box<dyn Fn(&mut S)>,
}

impl<S> HeaderAction<S> {
    pub fn new(label: impl Into<String>, on_click: impl Fn(&mut S) + 'static) -> Self {
        Self {
            label: label.into(),
            on_click: Box::new(on_click),
        }
    }
}

/// The status-bar text, derived from app state on every frame.
type StatusText<S> = Box<dyn Fn(&S) -> String>;

/// The shared layout. Build it with the `with`-style methods, then [`run`](Shell::run).
pub struct Shell<S> {
    title: String,
    theme: Theme,
    /// The product's own palette for the light side of the switch.
    light_palette: Palette,
    /// The product's own palette for the dark side of the switch.
    dark_palette: Palette,
    /// An optional brand accent applied over whichever palette is installed.
    accent: Option<Rgb>,
    screens: Vec<Box<dyn Screen<S>>>,
    actions: Vec<HeaderAction<S>>,
    status: Option<StatusText<S>>,
    nav: nav::NavState,
    nav_dir: Option<PathBuf>,
}

impl<S: 'static> Shell<S> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            theme: Theme::default(),
            light_palette: Palette::default(),
            dark_palette: Palette::default(),
            accent: None,
            screens: Vec::new(),
            actions: Vec::new(),
            status: None,
            nav: nav::NavState::default(),
            nav_dir: None,
        }
    }

    /// Palette to draw with. Apps read this from their settings file.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// The product's own palettes. `ui-shell` never invents brand colors; the
    /// app supplies them here, one for each side of the theme switch.
    pub fn palettes(mut self, light: Palette, dark: Palette) -> Self {
        self.light_palette = light;
        self.dark_palette = dark;
        self
    }

    /// Apply one brand accent over the product palettes. The canvas, text, and
    /// semantic roles stay the product's; only the accent family is derived.
    ///
    /// The shell installs visuals once, so a product that lets the user change
    /// the accent at runtime should use its own shell loop (as the POS does)
    /// rather than expecting this builder to repaint live.
    pub fn accent(mut self, accent: Rgb) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Register a screen, in sidebar order.
    pub fn screen(mut self, screen: impl Screen<S> + 'static) -> Self {
        self.screens.push(Box::new(screen));
        self
    }

    /// Add a header button. Actions are drawn right-to-left in registration order.
    pub fn header_action(
        mut self,
        label: impl Into<String>,
        on_click: impl Fn(&mut S) + 'static,
    ) -> Self {
        self.actions.push(HeaderAction::new(label, on_click));
        self
    }

    /// Add a status bar fed from app state, e.g. connection or cart state.
    pub fn status_bar(mut self, text: impl Fn(&S) -> String + 'static) -> Self {
        self.status = Some(Box::new(text));
        self
    }

    /// Remember the active screen in `<dir>/shell.json`.
    ///
    /// Pass `ctx.dirs.config` from `app-boot`. Window geometry is deliberately
    /// *not* persisted here — that is a separate concern and would pull in
    /// eframe's `persistence` feature.
    pub fn persist_in(mut self, dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        self.nav = nav::NavState::load(Some(&dir));
        self.nav_dir = Some(dir);
        self
    }

    /// Hand control to eframe. Blocks until the last window closes.
    ///
    /// Returns [`std::io::Error`] rather than `eframe::Error` on purpose:
    /// `eframe::Error` holds raw window handles and is therefore not `Sync`,
    /// so it cannot be `?`-converted into `anyhow::Error` by an app's `main`.
    /// The display message is preserved verbatim.
    pub fn run(self, state: S) -> std::io::Result<()> {
        assert!(
            !self.screens.is_empty(),
            "ui-shell: register at least one screen with .screen(..)"
        );

        let mut app = ShellApp { shell: self, state };

        // A remembered index can outlive the screen it pointed at.
        if app.shell.nav.active >= app.shell.screens.len() {
            app.shell.nav.active = 0;
        }

        let options = crate::native_options(
            egui::ViewportBuilder::default()
                .with_inner_size([1180.0, 760.0])
                .with_min_inner_size([880.0, 560.0]),
        );

        let title = app.shell.title.clone();
        eframe::run_native(
            &title,
            options,
            Box::new(
                move |cc| -> Result<Box<dyn eframe::App>, Box<dyn Error + Send + Sync>> {
                    let (palette, dark) = match app.shell.theme {
                        Theme::Light => (&app.shell.light_palette, false),
                        Theme::Dark => (&app.shell.dark_palette, true),
                    };
                    let palette = app
                        .shell
                        .accent
                        .map_or(*palette, |accent| palette.with_accent(accent, dark));
                    ui_kit::theme::install(&cc.egui_ctx, &palette, dark);
                    Ok(Box::new(app))
                },
            ),
        )
        .map_err(|err| std::io::Error::other(err.to_string()))
    }
}

struct ShellApp<S: 'static> {
    shell: Shell<S>,
    state: S,
}

impl<S: 'static> eframe::App for ShellApp<S> {
    // egui 0.36 hands the app a root `Ui` instead of a `Context`; panels are
    // nested inside it rather than painted onto the context directly.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Destructure first: the closures below need `state` mutably while
        // borrowing individual shell fields, which the borrow checker only
        // allows through disjoint field borrows.
        let Shell {
            title,
            screens,
            actions,
            status,
            nav,
            nav_dir,
            ..
        } = &mut self.shell;
        let state = &mut self.state;
        let mut nav_dirty = false;

        egui::Panel::top("shell_header").show(ui, |ui| {
            ui.add_space(ui_kit::tokens::SPACING.sm);
            ui.horizontal(|ui| {
                ui.label(
                    ui_kit::components::semibold(title.as_str()).size(ui_kit::tokens::TYPE.heading),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for action in actions.iter() {
                        if ui.button(action.label.as_str()).clicked() {
                            (action.on_click)(state);
                        }
                    }
                });
            });
            ui.add_space(ui_kit::tokens::SPACING.sm);
        });

        if let Some(status) = status.as_ref() {
            egui::Panel::bottom("shell_status").show(ui, |ui| {
                ui.add_space(ui_kit::tokens::SPACING.xs);
                ui.label(egui::RichText::new(status(state)).small().weak());
            });
        }

        egui::Panel::left("shell_nav")
            .resizable(true)
            .default_size(regions::size::SIDEBAR)
            .min_size(regions::size::SIDEBAR_MIN)
            .max_size(regions::size::SIDEBAR_MAX)
            .show(ui, |ui| {
                ui.add_space(ui_kit::tokens::SPACING.sm);
                for (index, screen) in screens.iter().enumerate() {
                    let label = match screen.icon() {
                        Some(icon) => format!("{icon}  {}", screen.title()),
                        None => screen.title().to_owned(),
                    };
                    if regions::nav_item(ui, nav.active == index, &label).clicked()
                        && nav.active != index
                    {
                        nav.active = index;
                        nav_dirty = true;
                    }
                }
            });

        regions::workspace(ui, |ui| {
            screens[nav.active].draw(ui, state);
        });

        // Saved on change rather than on exit: eframe's exit hook is tied to
        // the glow backend, and polling it is not worth the coupling.
        if nav_dirty {
            nav.save(nav_dir.as_deref());
        }
    }
}
