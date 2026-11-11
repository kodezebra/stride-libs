//! Named regions for products that arrange their own frame.
//!
//! [`crate::Shell`] is the one-layout-fits-everyone path: header, navigation,
//! status, content, fixed order. A product with a different frame — POS wants an
//! activity bar, a contextual sidebar and a cart column — does not fork `Shell`.
//! It calls the regions below in the order it wants, and gets the same chrome,
//! the same hit sizes and the same collapse animation for free.
//!
//! # Ordering
//!
//! Every region claims space from the same [`egui::Ui`], so draw the pinned
//! edges first and the workspace last:
//!
//! ```ignore
//! regions::status(ui, |ui| { /* left cluster, then regions::status_end */ });
//! if let Some(next) = regions::rail(ui, &RAIL, self.module) { self.select(next); }
//! regions::sidebar(ui, Sidebar::default(), &mut self.sidebar_open, |ui| { ... });
//! regions::workspace(ui, |ui| { ... });
//! ```
//!
//! # What is *not* here
//!
//! The cart, the product grid, the receipt list: those belong to one product's
//! workspace. A region is shared chrome, and shared chrome that grows product
//! vocabulary stops being shared.

use ui_kit::icons::Icon;
use ui_kit::theme::Theme;
use ui_kit::tokens::{RADII, SPACING, TYPE};

/// Fixed geometry, in points. Named so a product can align its own content to
/// the chrome without guessing numbers.
pub mod size {
    /// Width of the activity bar.
    pub const RAIL: f32 = 64.0;
    /// Height of one activity-bar entry. Deliberately ≥ 44: a till is used
    /// standing up, often with a finger on a touchscreen.
    pub const RAIL_ROW: f32 = 52.0;
    /// Icon size in the activity bar.
    pub const RAIL_ICON: f32 = 24.0;
    /// Height of the status bar.
    pub const STATUS: f32 = 36.0;
    /// Icon size in the status bar.
    pub const STATUS_ICON: f32 = 18.0;
    /// Default sidebar width.
    pub const SIDEBAR: f32 = 248.0;
    /// Narrowest the sidebar may be dragged.
    pub const SIDEBAR_MIN: f32 = 208.0;
    /// Widest the sidebar may be dragged.
    pub const SIDEBAR_MAX: f32 = 360.0;
    /// Height of one navigation row in a sidebar.
    pub const NAV_ROW: f32 = 36.0;
}

// ---------------------------------------------------------------------------
// Activity bar
// ---------------------------------------------------------------------------

/// One entry in the activity bar.
///
/// `label` and `hotkey` are shown in the tooltip. The hotkey is declared once,
/// next to the module, and both the tooltip and the key router read it — a
/// cashier must never find that the tooltip promises a key that does nothing.
pub struct RailItem {
    pub icon: Icon,
    pub label: &'static str,
    pub hotkey: &'static str,
}

/// Draw the activity bar and return the index of a clicked entry, if any.
pub fn rail(ui: &mut egui::Ui, items: &[RailItem], selected: usize) -> Option<usize> {
    let mut clicked = None;

    egui::Panel::left("shell_rail")
        .exact_size(size::RAIL)
        .resizable(false)
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
            for (index, item) in items.iter().enumerate() {
                if rail_entry(ui, item, index == selected) {
                    clicked = Some(index);
                }
            }
        });

    clicked
}

fn rail_entry(ui: &mut egui::Ui, item: &RailItem, selected: bool) -> bool {
    let palette = ui_kit::theme::current(ui.ctx());
    let accent_ink = palette.accent_ink();
    let (full, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), size::RAIL_ROW),
        egui::Sense::click(),
    );
    // Inset the painted selection so it reads as a chip inside the rail rather
    // than a full-bleed stripe glued to the window edge.
    let rect = full.shrink2(egui::vec2(8.0, 2.0));

    if ui.is_rect_visible(full) {
        let fill = if selected {
            egui::Color32::from(palette.accent_soft)
        } else if response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };

        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(RADII.md as u8), fill);
        if selected {
            // The 3 px accent edge is what makes the active module readable from
            // across the counter, where the icon alone is too small to judge.
            let edge = egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height()));
            painter.rect_filled(edge, egui::CornerRadius::same(2), accent_ink);
        }

        let color = if selected {
            egui::Color32::from(accent_ink)
        } else {
            ui.visuals().text_color()
        };
        let icon_rect =
            egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(size::RAIL_ICON));
        // `paint_at` rasterises the SVG at the exact pixel size, so it stays
        // crisp on a HiDPI till display instead of being scaled up.
        item.icon
            .image(size::RAIL_ICON, color)
            .paint_at(ui, icon_rect);
    }

    response
        .on_hover_text(format!("{}  ({})", item.label, item.hotkey))
        .clicked()
}

// ---------------------------------------------------------------------------
// Contextual sidebar
// ---------------------------------------------------------------------------

/// How wide the sidebar should be, and whether the user may drag it.
pub struct Sidebar {
    width: f32,
    resizable: bool,
}

impl Sidebar {
    /// A sidebar the user may drag between [`size::SIDEBAR_MIN`] and
    /// [`size::SIDEBAR_MAX`].
    pub fn new(width: f32) -> Self {
        Self {
            width,
            resizable: true,
        }
    }

    /// A fixed-width sidebar: no drag handle.
    ///
    /// Right for the till. A resize gesture that starts as a miss on a category
    /// button and ends as a 40 px sidebar is a mistake the cashier pays for.
    pub fn fixed(mut self) -> Self {
        self.resizable = false;
        self
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new(size::SIDEBAR)
    }
}

/// Draw the contextual sidebar, sliding it in and out with `open`.
///
/// `open` is taken by `&mut` because dragging the panel shut flips it — the
/// caller persists it afterwards and gets `Ctrl+B` behaviour without writing any
/// animation code.
pub fn sidebar(ui: &mut egui::Ui, spec: Sidebar, open: &mut bool, add: impl FnOnce(&mut egui::Ui)) {
    egui::Panel::left("shell_sidebar")
        .default_size(spec.width)
        .size_range(size::SIDEBAR_MIN..=size::SIDEBAR_MAX)
        .resizable(spec.resizable)
        .show_collapsible(ui, open, add);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

/// Draw the status bar. Content is a closure so a product shows its own truth
/// (queue depth, cashier, terminal) without the shell learning about it.
pub fn status(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Panel::bottom("shell_status")
        .exact_size(size::STATUS)
        .resizable(false)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = SPACING.lg;
                add(ui);
            });
        });
}

/// One status segment: an icon and a short label.
pub fn status_item(ui: &mut egui::Ui, icon: Icon, text: &str, color: egui::Color32) {
    // The tail of the bar runs right-to-left, which would otherwise paint the
    // label before the icon. Add them in paint order and use the gap the
    // bar's width estimate assumes.
    let right_to_left = ui.layout().main_dir() == egui::Direction::RightToLeft;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING.sm;
        if right_to_left {
            ui.label(egui::RichText::new(text).size(TYPE.status).color(color));
        }
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::splat(size::STATUS_ICON), egui::Sense::hover());
        icon.image(size::STATUS_ICON, color).paint_at(ui, rect);
        if !right_to_left {
            ui.label(egui::RichText::new(text).size(TYPE.status).color(color));
        }
    });
}

/// Right-aligned tail of the status bar.
///
/// Layout runs right-to-left here, so register segments in the reverse of the
/// order they should appear.
pub fn status_end(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add);
}

// ---------------------------------------------------------------------------
// Navigation rows
// ---------------------------------------------------------------------------

/// One sidebar navigation row: full width, inset, rounded, with an accent wash
/// when active.
///
/// `selectable_label` paints edge to edge and cannot take a height, which is
/// what made a product sidebar look like a list of links rather than a set of
/// destinations. This is the shared replacement: same row everywhere, one
/// definition, so the shell and a product's own sidebar agree.
pub fn nav_item(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let palette = ui_kit::theme::current(ui.ctx());
    let accent_ink = palette.accent_ink();
    let (full, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), size::NAV_ROW),
        egui::Sense::click(),
    );
    let rect = full.shrink2(egui::vec2(2.0, 1.0));

    if ui.is_rect_visible(full) {
        let fill = if selected {
            egui::Color32::from(palette.accent_soft)
        } else if response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        let color = if selected {
            egui::Color32::from(accent_ink)
        } else {
            ui.visuals().text_color()
        };

        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(RADII.sm as u8), fill);
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(TYPE.body),
            color,
        );
        painter.galley(
            egui::pos2(
                rect.min.x + SPACING.md,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            color,
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

/// The central panel: everything left over after the pinned edges.
///
/// The inner margin is the difference between a page that looks designed and
/// content glued to the window frame. The horizontal inset is deliberately the
/// widest step on the scale: right-aligned header actions (`New product`,
/// `Export CSV`) otherwise land against the scrollbar and read as clipped, and
/// long card grids lose their outer gutter.
pub fn workspace(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::central_panel(ui.style())
                .inner_margin(egui::Margin::symmetric(SPACING.xl as i8, SPACING.lg as i8)),
        )
        .show(ui, add);
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

/// One entry in the command palette.
pub struct Command<S> {
    /// What the user reads and types against.
    pub label: String,
    /// Optional right-hand hint, e.g. `"F2"` or `"Ctrl+P"`.
    pub hint: String,
    run: Box<dyn Fn(&mut S)>,
}

impl<S> Command<S> {
    pub fn new(label: impl Into<String>, run: impl Fn(&mut S) + 'static) -> Self {
        Self {
            label: label.into(),
            hint: String::new(),
            run: Box::new(run),
        }
    }

    /// Attach the shortcut already bound to this action, so the palette
    /// teaches the faster path instead of hiding it.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
}

/// Palette state: whether it is showing, what is typed, what is selected.
///
/// Owned by the product so the palette itself needs no globals and two windows
/// in one process can never fight over a query.
#[derive(Default)]
pub struct Palette {
    visible: bool,
    query: String,
    selected: usize,
    focus_pending: bool,
}

impl Palette {
    pub fn is_open(&self) -> bool {
        self.visible
    }

    /// Show the palette, clearing any previous query.
    pub fn request_open(&mut self) {
        if !self.visible {
            self.visible = true;
            self.query.clear();
            self.selected = 0;
            self.focus_pending = true;
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Draw the palette if it is open. Runs the chosen command and closes.
    pub fn show<S>(&mut self, ctx: &egui::Context, commands: &[Command<S>], state: &mut S) {
        if !self.visible {
            return;
        }

        let mut close = false;
        let mut chosen = None;

        let modal = egui::Modal::new(egui::Id::new("shell_palette")).show(ctx, |ui| {
            ui.set_width(520.0);

            // The same field the rest of the app uses: a palette with a
            // smaller, plainer input than every other screen reads as a
            // different program.
            let entry = ui_kit::components::search_field(ui, &mut self.query, "Type a command");
            if self.focus_pending {
                entry.request_focus();
                self.focus_pending = false;
            }

            let needle = self.query.trim().to_lowercase();
            let matches: Vec<usize> = commands
                .iter()
                .enumerate()
                .filter(|(_, command)| {
                    needle.is_empty() || command.label.to_lowercase().contains(&needle)
                })
                .map(|(index, _)| index)
                .collect();

            if matches.is_empty() {
                self.selected = 0;
            } else {
                self.selected = self.selected.min(matches.len() - 1);
            }

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !matches.is_empty() {
                self.selected = (self.selected + 1) % matches.len();
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !matches.is_empty() {
                self.selected = (self.selected + matches.len() - 1) % matches.len();
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                chosen = matches.get(self.selected).copied();
            }

            ui.add_space(4.0);
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    if matches.is_empty() {
                        ui.label(egui::RichText::new("No matching command").weak());
                        return;
                    }
                    for (row, index) in matches.iter().enumerate() {
                        if palette_row(ui, &commands[*index], row == self.selected) {
                            chosen = Some(*index);
                        }
                    }
                });

            ui.add_space(SPACING.xs);
            ui.label(
                egui::RichText::new("Up/Down select  -  Enter run  -  Esc close")
                    .size(TYPE.small)
                    .color(ui_kit::theme::current(ui.ctx()).text_muted),
            );
        });

        if modal.backdrop_response.clicked() {
            close = true;
        }

        if let Some(index) = chosen {
            (commands[index].run)(state);
            close = true;
        }

        if close {
            self.visible = false;
        }
    }
}

/// One palette row: label left, its shortcut right, both in the same card
/// language as the rest of the shell. The hint is visible rather than a tooltip
/// because the point of a palette is to teach the faster path.
fn palette_row<S>(ui: &mut egui::Ui, command: &Command<S>, selected: bool) -> bool {
    let palette = ui_kit::theme::current(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if selected {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(RADII.sm as u8),
                egui::Color32::from(palette.accent_soft),
            );
        } else if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(RADII.sm as u8),
                egui::Color32::from(palette.surface_alt),
            );
        }

        let inset = 10.0;
        painter.text(
            egui::pos2(rect.left() + inset, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &command.label,
            egui::FontId::new(TYPE.body, egui::FontFamily::Proportional),
            if selected {
                egui::Color32::from(palette.accent_ink())
            } else {
                egui::Color32::from(palette.text)
            },
        );

        if !command.hint.is_empty() {
            painter.text(
                egui::pos2(rect.right() - inset, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &command.hint,
                egui::FontId::new(TYPE.small, egui::FontFamily::Monospace),
                egui::Color32::from(palette.text_muted),
            );
        }
    }

    response.clicked()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Window geometry for [`run_app`].
pub struct Window {
    pub inner: [f32; 2],
    pub min: [f32; 2],
}

impl Default for Window {
    /// Sized so the window still fits a 1366×768 till without the window manager
    /// clamping it, which looks like a bug to the person using it.
    fn default() -> Self {
        Self {
            inner: [1240.0, 720.0],
            min: [960.0, 600.0],
        }
    }
}

struct LooseApp<S, F> {
    state: S,
    draw: F,
}

impl<S: 'static, F: FnMut(&mut egui::Ui, &mut S) + 'static> eframe::App for LooseApp<S, F> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        (self.draw)(ui, &mut self.state);
    }
}

/// Hand control to eframe, letting `draw` own the whole frame.
///
/// This is the counterpart to [`crate::Shell::run`] for products that arrange
/// their own regions. The theme is installed once, here, for the same reason the
/// shell does it: screens must never touch global visuals. The app supplies its
/// own palettes — `ui-shell` never invents brand colors.
pub fn run_app<S, F>(
    title: &str,
    theme: Theme,
    light_palette: ui_kit::tokens::Palette,
    dark_palette: ui_kit::tokens::Palette,
    window: Window,
    state: S,
    draw: F,
) -> std::io::Result<()>
where
    S: 'static,
    F: FnMut(&mut egui::Ui, &mut S) + 'static,
{
    let options = crate::native_options(
        egui::ViewportBuilder::default()
            .with_inner_size(window.inner)
            .with_min_inner_size(window.min),
    );

    let app = LooseApp { state, draw };

    eframe::run_native(
        title,
        options,
        Box::new(
            move |cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
                let (palette, dark) = match theme {
                    Theme::Light => (&light_palette, false),
                    Theme::Dark => (&dark_palette, true),
                };
                ui_kit::theme::install(&cc.egui_ctx, palette, dark);
                Ok(Box::new(app))
            },
        ),
    )
    .map_err(|err| std::io::Error::other(err.to_string()))
}
