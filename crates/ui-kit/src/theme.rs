//! Theme selection and palette application.
//!
//! `ui-kit` owns the mechanism — the `Theme` selector, the `Palette` struct,
//! and applying a palette to an egui context. Each product owns its own palette
//! values: a coffee shop till, a school admin, and a clinic desk all look
//! different, and none of that brand domain belongs here.

use crate::tokens::Palette;

/// Which palette an app is drawn with.
///
/// Stored as a string in each app's settings file (see the `config` crate), so
/// changing the on-disk spelling is a breaking change for existing installs.
///
/// The app maps this to its own palette: [`Theme::Light`] is not "the light
/// palette", it is "the light side of the switch", and each product decides
/// what colors that means.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// The same value as [`Theme::name`], written the way it is shown to a
    /// person. `name` is the settings-file spelling and must not change.
    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }

    /// Unknown values fall back to the default theme rather than erroring: a
    /// hand-edited settings file should not stop an app from starting. A file
    /// that says `"dark"` keeps opening dark.
    pub fn parse(value: &str) -> Self {
        match value {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }

    /// The other theme, for a two-way appearance switch.
    pub fn flipped(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

/// Apply a palette to an egui context.
///
/// Called once at startup by `ui-shell` or a product's own shell. A product's
/// shell may call it again when the user changes the appearance setting — the
/// shell is the only place allowed to touch global visuals; screens never do.
///
/// `palette` is the product's own color set for this theme; `dark` tells egui
/// whether to use dark-mode chrome. The two are independent: a product could
/// ship a dark theme with a light palette or vice versa.
#[cfg(feature = "widgets")]
pub fn install(ctx: &egui::Context, palette: &Palette, dark: bool) {
    install_fonts(ctx);
    install_style(ctx, palette, dark);
    install_image_loaders(ctx);

    // Remember the palette so widgets can read it without every call site
    // threading a `&Palette` through. `ctx.data` is per-context, so two
    // windows in one process cannot disagree.
    ctx.data_mut(|data| data.insert_temp(palette_id(), *palette));
}

/// Install a product palette with one brand accent, keeping the palette's own
/// canvas and semantic colours.
///
/// This is the runtime counterpart to [`Palette::with_accent`]; an app that
/// stores a brand choice can call this without rebuilding the whole palette.
#[cfg(feature = "widgets")]
pub fn install_accent(
    ctx: &egui::Context,
    palette: &Palette,
    accent: crate::tokens::Rgb,
    dark: bool,
) {
    install(ctx, &palette.with_accent(accent, dark), dark);
}

/// The palette the current context was installed with.
#[cfg(feature = "widgets")]
pub fn current(ctx: &egui::Context) -> Palette {
    ctx.data(|data| data.get_temp::<Palette>(palette_id()))
        .unwrap_or_default()
}

#[cfg(feature = "widgets")]
fn palette_id() -> egui::Id {
    egui::Id::new("ui_kit_palette")
}

/// The semibold family: Open Sans Semibold. Use through [`semibold`] so screens
/// never name a font.
#[cfg(feature = "widgets")]
pub fn semibold() -> egui::FontFamily {
    egui::FontFamily::Name("semibold".into())
}

/// The numeric family: JetBrains Mono, for money columns and receipts.
#[cfg(feature = "widgets")]
pub fn numeric() -> egui::FontFamily {
    egui::FontFamily::Name("numeric".into())
}

#[cfg(feature = "widgets")]
fn install_fonts(ctx: &egui::Context) {
    use std::sync::Arc;

    let mut fonts = egui::FontDefinitions::default();

    // TTF rather than OTF: epaint reads TrueType outlines directly, and the
    // CFF outlines in Open Sans' OTF release are not supported.
    fonts.font_data.insert(
        "open-sans".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/OpenSans-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "open-sans-semibold".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/OpenSans-Semibold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains-mono".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    // Open Sans leads; egui's bundled fonts stay behind it as fallbacks, so a
    // glyph Open Sans lacks (or an emoji) still renders instead of a tofu box.
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, "open-sans".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.insert(0, "jetbrains-mono".to_owned());
    }

    let proportional = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let mut semibold = vec!["open-sans-semibold".to_owned()];
    semibold.extend(proportional);
    fonts
        .families
        .insert(egui::FontFamily::Name("semibold".into()), semibold);

    let mut numeric = vec!["jetbrains-mono".to_owned()];
    numeric.extend(monospace);
    fonts
        .families
        .insert(egui::FontFamily::Name("numeric".into()), numeric);

    ctx.set_fonts(fonts);
}

#[cfg(feature = "widgets")]
fn install_style(ctx: &egui::Context, palette: &Palette, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.dark_mode = dark;
    visuals.panel_fill = palette.bg.into();
    visuals.window_fill = palette.surface.into();
    visuals.faint_bg_color = palette.surface_alt.into();
    visuals.extreme_bg_color = palette.surface.into();
    visuals.text_edit_bg_color = Some(palette.surface.into());
    visuals.code_bg_color = palette.surface_alt.into();
    visuals.override_text_color = Some(palette.text.into());
    visuals.weak_text_color = Some(palette.text_muted.into());
    visuals.hyperlink_color = palette.accent_ink().into();
    visuals.warn_fg_color = palette.warning.into();
    visuals.error_fg_color = palette.danger.into();

    // Text selection and selected buttons share these two roles. Keeping the
    // wash pale and the text accent means a selected row reads as "active"
    // without turning into a solid block of brand colour. The ink is derived
    // from contrast, because the raw accent is not always legible as text.
    let accent_ink = palette.accent_ink();
    visuals.selection.bg_fill = palette.accent_soft.into();
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent_ink);

    visuals.window_corner_radius = egui::CornerRadius::same(crate::tokens::RADII.lg as u8);
    visuals.menu_corner_radius = egui::CornerRadius::same(crate::tokens::RADII.md as u8);
    visuals.window_stroke = egui::Stroke::new(1.0_f32, palette.border);

    let corner = egui::CornerRadius::same(crate::tokens::RADII.sm as u8);
    let hairline = egui::Stroke::new(1.0_f32, palette.border);
    let text = egui::Stroke::new(1.0_f32, palette.text);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = palette.surface.into();
    widgets.noninteractive.weak_bg_fill = palette.surface.into();
    widgets.noninteractive.bg_stroke = hairline;
    widgets.noninteractive.fg_stroke = text;
    widgets.noninteractive.corner_radius = corner;

    widgets.inactive.bg_fill = palette.surface_alt.into();
    widgets.inactive.weak_bg_fill = palette.surface.into();
    widgets.inactive.bg_stroke = hairline;
    widgets.inactive.fg_stroke = text;
    widgets.inactive.corner_radius = corner;

    widgets.hovered.bg_fill = palette.surface_alt.into();
    widgets.hovered.weak_bg_fill = palette.accent_soft.into();
    widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, accent_ink);
    widgets.hovered.fg_stroke = text;
    widgets.hovered.corner_radius = corner;

    widgets.active.bg_fill = palette.accent_soft.into();
    widgets.active.weak_bg_fill = palette.accent_soft.into();
    widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, accent_ink);
    widgets.active.fg_stroke = text;
    widgets.active.corner_radius = corner;

    widgets.open = widgets.active;

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                egui::TextStyle::Small,
                egui::FontId::proportional(crate::tokens::TYPE.small),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::proportional(crate::tokens::TYPE.body),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::proportional(crate::tokens::TYPE.button),
            ),
            (
                egui::TextStyle::Heading,
                egui::FontId::proportional(crate::tokens::TYPE.heading),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(crate::tokens::TYPE.receipt, egui::FontFamily::Monospace),
            ),
        ]
        .into();

        // A till is used standing up, often with a finger: controls are taller
        // than egui's desktop defaults on purpose.
        style.spacing.item_spacing = egui::vec2(crate::tokens::SPACING.sm, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.interact_size = egui::vec2(44.0, 36.0);
    });

    // Both egui themes get the product palette, so a change to the desktop's
    // light/dark preference cannot repaint the app in a foreign colour scheme.
    ctx.set_visuals_of(egui::Theme::Light, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Dark, visuals);
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

#[cfg(feature = "icons")]
fn install_image_loaders(ctx: &egui::Context) {
    // `install` runs again on a theme switch; stacking a second set of loaders
    // would grow the loader list on every click.
    let marker = egui::Id::new("ui_kit_image_loaders");
    let already = ctx.data(|data| data.get_temp::<bool>(marker).unwrap_or(false));
    if already {
        return;
    }
    egui_extras::install_image_loaders(ctx);
    ctx.data_mut(|data| data.insert_temp(marker, true));
}

#[cfg(all(feature = "widgets", not(feature = "icons")))]
fn install_image_loaders(_ctx: &egui::Context) {}
