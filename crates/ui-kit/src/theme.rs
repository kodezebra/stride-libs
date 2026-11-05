//! Theme selection, palette values, and the one place fonts are installed.

use crate::tokens::{Palette, RADII, Rgb, SPACING, TYPE};

/// Which palette an app is drawn with.
///
/// Stored as a string in each app's settings file (see the `config` crate), so
/// changing the on-disk spelling is a breaking change for existing installs.
///
/// Light is the default: a till sits under shop lighting next to a window, and
/// a dark screen reads as a dead terminal to a customer standing at the counter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn palette(self) -> Palette {
        match self {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        }
    }

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

/// Dark theme — Scandinavian FinTech.
///
/// Slate-obsidian shell, muted slate surfaces, electric-mint primary actions.
/// Cool, calm, hyper-legible for low-light hours; the mint stays visually sharp
/// without the optical intensity of saturated orange or blue.
const DARK: Palette = Palette {
    bg: Rgb::hex(0x0f_17_2a),
    surface: Rgb::hex(0x1e_29_3b),
    surface_alt: Rgb::hex(0x33_41_55),
    text: Rgb::hex(0xf8_fa_fc),
    text_muted: Rgb::hex(0x94_a3_b8),
    accent: Rgb::hex(0x14_b8_a6),
    accent_text: Rgb::hex(0x0f_17_2a),
    accent_soft: Rgb::hex(0x16_4e_4b),
    border: Rgb::hex(0x33_41_55),
    border_strong: Rgb::hex(0x64_74_8b),
    danger: Rgb::hex(0xef_44_44),
    success: Rgb::hex(0x34_d3_99),
    warning: Rgb::hex(0xf5_9e_0b),
};

/// Light theme — Artisanal Coffee & Boutique Retail.
///
/// Warm cashmere-cream canvas, deep warm-charcoal shell, terracotta primary.
/// Organic and low-blue-light, easy on the eyes in bright retail, and nothing
/// like corporate POS software. The default: a till under shop lighting next to
/// a window, where a dark screen reads as a dead terminal to a customer.
const LIGHT: Palette = Palette {
    bg: Rgb::hex(0xf5_f5_f4),
    surface: Rgb::hex(0xff_ff_ff),
    surface_alt: Rgb::hex(0xe7_e5_e4),
    text: Rgb::hex(0x1c_19_17),
    text_muted: Rgb::hex(0x6e_66_5d),
    accent: Rgb::hex(0xc2_41_0c),
    accent_text: Rgb::hex(0xff_ff_ff),
    accent_soft: Rgb::hex(0xfd_ec_e6),
    border: Rgb::hex(0xd6_d3_ce),
    border_strong: Rgb::hex(0xb9_b4_ac),
    danger: Rgb::hex(0xc0_39_2b),
    success: Rgb::hex(0x84cc16),
    warning: Rgb::hex(0xd9_77_06),
};

/// Apply a palette to an egui context.
///
/// Called once at startup by `ui-shell`. A product's shell may call it again
/// when the user changes the appearance setting — the shell is the only place
/// allowed to touch global visuals; screens never do.
#[cfg(feature = "widgets")]
pub fn install(ctx: &egui::Context, theme: Theme) {
    install_fonts(ctx);
    install_style(ctx, theme);
    install_image_loaders(ctx);

    // Remember the choice so widgets can read palette roles without every call
    // site threading a `Theme` through. `ctx.data` is per-context, so two
    // windows in one process cannot disagree.
    ctx.data_mut(|data| data.insert_temp(theme_id(), theme));
}

/// The palette the current context was installed with.
#[cfg(feature = "widgets")]
pub fn current(ctx: &egui::Context) -> Palette {
    ctx.data(|data| data.get_temp::<Theme>(theme_id()))
        .unwrap_or_default()
        .palette()
}

#[cfg(feature = "widgets")]
fn theme_id() -> egui::Id {
    egui::Id::new("ui_kit_theme")
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
fn install_style(ctx: &egui::Context, theme: Theme) {
    let palette = theme.palette();
    let dark = matches!(theme, Theme::Dark);

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
    visuals.hyperlink_color = palette.accent.into();
    visuals.warn_fg_color = palette.warning.into();
    visuals.error_fg_color = palette.danger.into();

    // Text selection and selected buttons share these two roles. Keeping the
    // wash pale and the text accent means a selected row reads as "active"
    // without turning into a solid block of brand colour.
    visuals.selection.bg_fill = palette.accent_soft.into();
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, palette.accent);

    visuals.window_corner_radius = egui::CornerRadius::same(RADII.lg as u8);
    visuals.menu_corner_radius = egui::CornerRadius::same(RADII.md as u8);
    visuals.window_stroke = egui::Stroke::new(1.0_f32, palette.border);

    let corner = egui::CornerRadius::same(RADII.sm as u8);
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
    widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, palette.accent);
    widgets.hovered.fg_stroke = text;
    widgets.hovered.corner_radius = corner;

    widgets.active.bg_fill = palette.accent_soft.into();
    widgets.active.weak_bg_fill = palette.accent_soft.into();
    widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, palette.accent);
    widgets.active.fg_stroke = text;
    widgets.active.corner_radius = corner;

    widgets.open = widgets.active;

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                egui::TextStyle::Small,
                egui::FontId::proportional(TYPE.small),
            ),
            (egui::TextStyle::Body, egui::FontId::proportional(TYPE.body)),
            (
                egui::TextStyle::Button,
                egui::FontId::proportional(TYPE.button),
            ),
            (
                egui::TextStyle::Heading,
                egui::FontId::proportional(TYPE.heading),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(TYPE.receipt, egui::FontFamily::Monospace),
            ),
        ]
        .into();

        // A till is used standing up, often with a finger: controls are taller
        // than egui's desktop defaults on purpose.
        style.spacing.item_spacing = egui::vec2(SPACING.sm, 6.0);
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
