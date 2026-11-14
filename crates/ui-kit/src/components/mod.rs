//! Reusable widget primitives.
//!
//! These are building blocks, not a component framework: a handful of small
//! functions that keep spacing, weight, and colour consistent across products.
//! Anything that needs state or knows a domain concept belongs in an app, and
//! anything that arranges a whole page belongs in `ui-shell`.
//!
//! Two rules hold everywhere in this module:
//!
//! * Colours come from [`crate::theme::current`], never from the egui style.
//!   The style's selection colours are shared with text selection, so reading
//!   them for a button would tie two unrelated decisions together.
//! * Type sizes come from [`TYPE`], never from a literal. If a size is missing
//!   from the scale, add it to the scale first.

use crate::theme;
use crate::tokens::{BRANDS, RADII, Rgb, SPACING, TYPE};
use egui::{
    Color32, CornerRadius, FontId, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
    Widget,
};

/// A section heading with the spacing that should follow it.
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.add_space(SPACING.md);
    ui.label(
        RichText::new(text)
            .family(theme::semibold())
            .size(TYPE.heading)
            .color(theme::current(ui.ctx()).text),
    );
    ui.add_space(SPACING.xs);
}

/// A raised surface: the container every group of controls should sit in.
///
/// One border, one radius, one margin. A card is what turns a page of loose
/// widgets into a panel a cashier can find at a glance.
///
/// The contents are laid out top to bottom even when the card itself sits in a
/// row: `Frame::show` otherwise inherits the parent's direction, and a caption
/// beside its number is not a card. Rows inside a card ask for `ui.horizontal`
/// explicitly.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let palette = theme::current(ui.ctx());
    egui::Frame::NONE
        .fill(palette.surface.into())
        .stroke(Stroke::new(1.0_f32, palette.border))
        .corner_radius(CornerRadius::same(RADII.md as u8))
        .inner_margin(egui::Margin::same(SPACING.lg as i8))
        .show(ui, |ui| ui.vertical(add).inner)
        .inner
}

/// A dashboard figure: caption above, number below, inside a card.
pub fn stat_card(ui: &mut Ui, label: &str, value: &str) {
    card(ui, |ui| {
        ui.set_min_width(128.0);
        field_label(ui, label);
        ui.add_space(SPACING.xs);
        ui.label(numeric(value).size(TYPE.kpi));
    });
}

/// A product tile. Drawn rather than composed from labels so the name can wrap
/// while the price stays pinned to the bottom edge.
pub struct Tile<'a> {
    pub name: &'a str,
    pub price: &'a str,
}

impl Widget for Tile<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let palette = theme::current(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::click());

        if ui.is_rect_visible(rect) {
            let hovered = response.hovered();
            let fill = if hovered {
                palette.surface_alt
            } else {
                palette.surface
            };
            let border = if hovered {
                palette.border_strong
            } else {
                palette.border
            };

            let painter = ui.painter();
            painter.rect(
                rect,
                CornerRadius::same(RADII.md as u8),
                fill,
                Stroke::new(1.0_f32, border),
                StrokeKind::Inside,
            );

            let inner = rect.shrink(SPACING.md);
            let name = painter.layout(
                self.name.to_owned(),
                FontId::new(TYPE.body, theme::semibold()),
                palette.text.into(),
                inner.width(),
            );
            painter.galley(inner.min, name, palette.text.into());

            let price = painter.layout_no_wrap(
                self.price.to_owned(),
                FontId::new(TYPE.small, theme::numeric()),
                palette.text_muted.into(),
            );
            painter.galley(
                egui::pos2(inner.min.x, inner.max.y - price.size().y),
                price,
                palette.text_muted.into(),
            );
        }

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// The caption above a control. Muted, small, and never bold: the control itself
/// carries the emphasis.
pub fn field_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(TYPE.small)
            .color(theme::current(ui.ctx()).text_muted),
    );
}

/// A full-width search or scan field, tall enough to hit without looking.
pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    let palette = theme::current(ui.ctx());
    let width = ui.available_width();
    ui.scope(|ui| {
        // Inputs get the strong border: they must stay visible against the card
        // they sit on, which a hairline does not guarantee at 3:1.
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.border_strong);
        // egui draws the focused TextEdit frame with `selection.stroke`. Make
        // that frame a real focus ring here rather than changing the global
        // width, which would also thicken every text selection.
        ui.visuals_mut().selection.stroke = Stroke::new(2.0_f32, palette.accent_ink());
        ui.add_sized(
            [width, 44.0],
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .margin(egui::Margin::symmetric(12, 12)),
        )
    })
    .inner
}

/// A small muted line: secondary information under a title or a table.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(TYPE.small)
            .color(theme::current(ui.ctx()).text_muted),
    );
}

/// A table column heading.
pub fn table_header(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(TYPE.small)
            .family(theme::semibold())
            .color(theme::current(ui.ctx()).text_muted),
    );
}

/// A rule between groups inside a card.
pub fn divider(ui: &mut Ui) {
    let palette = theme::current(ui.ctx());
    ui.add_space(SPACING.sm);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, palette.border),
    );
    ui.add_space(SPACING.sm);
}

/// Numeric text: money, quantities, receipt lines. Always monospaced so digits
/// line up in a column without a table.
pub fn numeric(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).family(theme::numeric())
}

/// Semibold text in the UI face, for labels that need weight without size.
pub fn semibold(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).family(theme::semibold())
}

/// Small pill label — "Draft", "Paid", "Synced". Colour comes from the palette
/// role the caller picked (`success`, `danger`, `text_muted`, …).
pub fn badge(ui: &mut Ui, text: &str, color: Rgb) {
    let palette = theme::current(ui.ctx());
    let foreground = Color32::from(color);
    let background = mix(palette.surface.into(), foreground, 0.14);
    egui::Frame::NONE
        .fill(background)
        .corner_radius(CornerRadius::same(u8::MAX))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(TYPE.small).color(foreground));
        });
}

/// Empty-state block: what is missing, and what to do about it.
///
/// Sits at the top of a section. Use [`empty_state_fill`] when the emptiness is
/// the whole pane.
pub fn empty_state(ui: &mut Ui, title: &str, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING.xl * 2.0);
        ui.label(semibold(title).size(TYPE.heading));
        ui.add_space(SPACING.xs);
        ui.label(RichText::new(hint).weak());
    });
}

/// The same message, centred in the space it is given.
///
/// For panes whose content *is* the empty state — an empty cart, a detail view
/// with nothing selected. A line of text at the top of a tall pane reads as
/// something that failed to load; centred, it reads as a quiet "nothing here
/// yet". The height comes from the font metrics rather than a guess, so the
/// message stays centred when the type scale changes.
pub fn empty_state_fill(ui: &mut Ui, title: &str, hint: &str) {
    let rect = ui.available_rect_before_wrap();
    let title_height = ui
        .ctx()
        .fonts_mut(|fonts| fonts.row_height(&FontId::new(TYPE.heading, theme::semibold())));
    let hint_height = ui.text_style_height(&egui::TextStyle::Body);
    let block = title_height + ui.spacing().item_spacing.y + SPACING.xs + hint_height;

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    child.add_space(((rect.height() - block) * 0.5).max(0.0));
    child.label(semibold(title).size(TYPE.heading));
    child.add_space(SPACING.xs);
    child.label(RichText::new(hint).weak());
}

/// An empty-state hint inside a card of fixed height, for a section that will
/// become a table once there are rows. The card is the point: a bare hint under
/// a heading reads as missing content, an empty container reads as "no rows yet".
pub fn empty_state_card(ui: &mut Ui, title: &str, hint: &str, height: f32) {
    card(ui, |ui| {
        let rect =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), height));
        let _ = ui.allocate_rect(rect, egui::Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        empty_state_fill(&mut child, title, hint);
    });
}

/// The one emphasised action per view. Never place two side by side — that is
/// how a UI ends up with no clear primary action.
pub fn primary_button(ui: &mut Ui, label: &str) -> Response {
    primary_action(ui, label, 44.0)
}

/// The same button at an explicit height. The till's Pay action is taller than
/// a dialog's confirm because it is hit without looking.
pub fn primary_action(ui: &mut Ui, label: &str, height: f32) -> Response {
    let palette = theme::current(ui.ctx());
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(TYPE.button)
                .family(theme::semibold())
                .color(palette.accent_text),
        )
        .fill(palette.accent)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(RADII.sm as u8))
        .min_size(egui::vec2(0.0, height)),
    )
}

/// A selectable card with a glyph and a label: one option in a small set, such
/// as the appearance switch.
///
/// Selected is drawn with the accent wash and an accent border, the same pair a
/// navigation row uses, so "chosen" means one thing across the app.
#[cfg(feature = "icons")]
pub fn choice_card(
    ui: &mut Ui,
    icon: crate::icons::Icon,
    label: &str,
    selected: bool,
    width: f32,
) -> Response {
    let palette = theme::current(ui.ctx());
    let height = 76.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if selected {
            Color32::from(palette.accent_soft)
        } else if response.hovered() {
            Color32::from(palette.surface_alt)
        } else {
            Color32::from(palette.surface)
        };
        let border = if selected {
            Stroke::new(2.0_f32, palette.accent_ink())
        } else if response.hovered() {
            Stroke::new(1.0_f32, palette.border_strong)
        } else {
            Stroke::new(1.0_f32, palette.border)
        };

        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(RADII.md as u8),
            fill,
            border,
            StrokeKind::Inside,
        );

        let color = if selected {
            Color32::from(palette.accent_ink())
        } else {
            Color32::from(palette.text)
        };
        let glyph = 22.0;
        let icon_center = egui::pos2(rect.center().x, rect.min.y + 26.0);
        icon.image(glyph, color)
            .paint_at(ui, Rect::from_center_size(icon_center, Vec2::splat(glyph)));

        let galley = painter.layout_no_wrap(
            label.to_owned(),
            FontId::new(TYPE.small, theme::semibold()),
            color,
        );
        painter.galley(
            egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.max.y - 12.0 - galley.size().y,
            ),
            galley,
            color,
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// One line of a receipt preview: label left, amount right, both monospaced.
pub fn receipt_line(ui: &mut Ui, label: &str, value: &str) {
    let palette = theme::current(ui.ctx());
    let color = Color32::from(palette.text);
    // Fixed height: a right-to-left child otherwise claims the rest of the pane,
    // which turns a short receipt into one line and a lot of blank paper.
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), TYPE.receipt * 1.8),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(
                RichText::new(value)
                    .family(theme::numeric())
                    .size(TYPE.receipt)
                    .color(color),
            );
            ui.label(
                RichText::new(label)
                    .family(theme::numeric())
                    .size(TYPE.receipt)
                    .color(color),
            );
        },
    );
}

/// A square icon button. Used for cart steppers and chrome actions.
#[cfg(feature = "icons")]
pub fn icon_button(ui: &mut Ui, icon: crate::icons::Icon, size: f32) -> Response {
    let palette = theme::current(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let fill = if hovered {
            Color32::from(palette.surface_alt)
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if hovered {
            Stroke::new(1.0_f32, palette.border_strong)
        } else {
            Stroke::NONE
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(RADII.sm as u8),
            fill,
            stroke,
            StrokeKind::Inside,
        );

        let color = if hovered {
            Color32::from(palette.accent_ink())
        } else {
            ui.visuals().text_color()
        };
        let glyph = size * 0.55;
        icon.image(glyph, color).paint_at(
            ui,
            Rect::from_center_size(rect.center(), Vec2::splat(glyph)),
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// What a [`stepper`] was asked to do this frame.
#[cfg(feature = "icons")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    None,
    Up,
    Down,
}

/// A quantity stepper: minus, count, plus.
///
/// Returns the change and the count's response rather than mutating anything,
/// so the caller owns the domain rule (a line at zero is removed, not shown as
/// zero).
///
/// The count is a button. A cashier selling twelve of something should not have
/// to tap plus eleven times, and the number is the thing they would tap to say
/// so — so it is framed like its neighbours instead of being a label that
/// happens to be clickable.
#[cfg(feature = "icons")]
pub fn stepper(ui: &mut Ui, qty: i64) -> (Step, Response) {
    let palette = theme::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING.xs;
        let mut step = Step::None;
        if step_button(ui, crate::icons::Icon::MINUS).clicked() {
            step = Step::Down;
        }
        let count = ui.add(
            egui::Button::new(numeric(qty.to_string()).size(TYPE.body))
                .min_size(Vec2::new(36.0, 30.0))
                .fill(Color32::from(palette.surface_alt))
                .stroke(Stroke::new(1.0_f32, palette.border))
                .corner_radius(CornerRadius::same(RADII.sm as u8)),
        );
        if step_button(ui, crate::icons::Icon::PLUS).clicked() {
            step = Step::Up;
        }
        (step, count)
    })
    .inner
}

/// A stepper key: visible at rest, because a cashier should not have to hover a
/// row to discover that the quantity is editable.
fn step_button(ui: &mut Ui, icon: crate::icons::Icon) -> Response {
    let palette = theme::current(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let fill = if hovered {
            Color32::from(palette.accent_soft)
        } else {
            Color32::from(palette.surface_alt)
        };
        let stroke = if hovered {
            Stroke::new(1.0_f32, palette.accent_ink())
        } else {
            Stroke::new(1.0_f32, palette.border)
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(RADII.sm as u8),
            fill,
            stroke,
            StrokeKind::Inside,
        );

        let color = if hovered {
            Color32::from(palette.accent_ink())
        } else {
            ui.visuals().text_color()
        };
        let glyph = 14.0;
        icon.image(glyph, color).paint_at(
            ui,
            Rect::from_center_size(rect.center(), Vec2::splat(glyph)),
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A row of one-click brand accents.
///
/// Returns `Some(None)` when the product accent is chosen, `Some(Some(accent))`
/// when a brand is chosen, and `None` when the user did not click. The return
/// value deliberately does not mutate a caller-owned colour: doing that would
/// make the comparison after the call always equal and the settings dirty flag
/// would never fire.
pub fn brand_selector(ui: &mut Ui, selected: Option<Rgb>) -> Option<Option<Rgb>> {
    let mut choice = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(SPACING.sm, SPACING.sm);
        if brand_swatch(ui, None, "Default", selected.is_none()).clicked() {
            choice = Some(None);
        }
        for brand in BRANDS {
            let is_selected = selected == Some(brand.accent);
            if brand_swatch(ui, Some(brand.accent), brand.name, is_selected).clicked() {
                choice = Some(Some(brand.accent));
            }
        }
    });
    choice
}

fn brand_swatch(ui: &mut Ui, accent: Option<Rgb>, label: &str, selected: bool) -> Response {
    let palette = theme::current(ui.ctx());
    let fill = accent.unwrap_or(palette.surface_alt);
    let ink = accent.map(Rgb::best_ink).unwrap_or(palette.text);
    let stroke = if selected {
        Stroke::new(2.0_f32, palette.text)
    } else {
        Stroke::new(1.0_f32, palette.border_strong)
    };

    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(TYPE.small)
                .family(theme::semibold())
                .color(ink),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(RADII.sm as u8))
        .min_size(Vec2::new(88.0, 38.0)),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Blend `b` into `a`. Used to tint a palette role against the surface it sits
/// on, so a badge stays legible in both themes without a second token.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}
