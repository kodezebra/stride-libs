//! Raw design tokens.
//!
//! Values are plain data with no UI-framework types, so the same numbers can
//! drive an egui app today and a web page, a PDF receipt, or an ESC/POS printer
//! later. Conversion to a backend's colour type happens at the edge — see the
//! `From<Rgb> for egui::Color32` impl behind the `widgets` feature.

/// An sRGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `0xRRGGBB` literal, convenient when copying values from a design file.
    pub const fn hex(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        }
    }

    /// The same colour as a `0xRRGGBB` integer.
    pub const fn to_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    /// Parse `#RRGGBB` or `RRGGBB`. Values in other formats are rejected rather
    /// than guessed: a settings file with a typo should keep the product accent,
    /// not load a near-miss colour.
    pub fn parse_hex(value: &str) -> Option<Self> {
        let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if value.len() != 6 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(value, 16).ok().map(Self::hex)
    }

    /// WCAG 2.x relative luminance. The channels must be linearised first;
    /// averaging gamma-encoded bytes is luma, not luminance, and gives the wrong
    /// answer for the middle of the scale (amber and orange are the usual
    /// casualties).
    pub fn relative_luminance(self) -> f32 {
        fn linear(channel: u8) -> f32 {
            let channel = f32::from(channel) / 255.0;
            if channel <= 0.040_45 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }

    /// WCAG contrast ratio, always in `1.0..=21.0`.
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let a = self.relative_luminance();
        let b = other.relative_luminance();
        let (lighter, darker) = if a > b { (a, b) } else { (b, a) };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Black or white, whichever reads better on this colour. Used for text on
    /// a brand fill: the answer is derived, never stored by hand.
    pub fn best_ink(self) -> Self {
        if Self::WHITE.contrast_ratio(self) >= Self::BLACK.contrast_ratio(self) {
            Self::WHITE
        } else {
            Self::BLACK
        }
    }

    /// Blend `other` into `self`; `amount` is clamped to `0.0..=1.0`.
    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let lerp =
            |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8;
        Self::new(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
        )
    }
}

#[cfg(feature = "widgets")]
impl From<Rgb> for egui::Color32 {
    fn from(color: Rgb) -> Self {
        egui::Color32::from_rgb(color.r, color.g, color.b)
    }
}

/// The full colour set for one theme. Adding a role here means every app must
/// supply it; keep this list small and semantic rather than literal.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Window background, behind panels.
    pub bg: Rgb,
    /// Raised surfaces: sidebars, cards, dialogs.
    pub surface: Rgb,
    /// Alternate surface: table stripes, inactive wells, code blocks.
    pub surface_alt: Rgb,
    pub text: Rgb,
    pub text_muted: Rgb,
    /// Primary action colour.
    pub accent: Rgb,
    /// Text drawn on top of `accent`.
    pub accent_text: Rgb,
    /// A pale accent wash: selected navigation rows, text selection. Subtle
    /// enough that accent-coloured text stays the thing that reads as "active".
    pub accent_soft: Rgb,
    /// Hairline between surfaces: card edges, table rules.
    pub border: Rgb,
    /// Outline for controls that must stay visible on their own (inputs,
    /// focusable boxes). Meets the 3:1 non-text contrast floor against both
    /// `surface` and `bg`.
    pub border_strong: Rgb,
    pub danger: Rgb,
    pub success: Rgb,
    /// Caution: unsynced queue, low stock. Distinct from `danger`.
    pub warning: Rgb,
}

impl Default for Palette {
    /// A neutral light fallback: used only if nothing has been installed yet.
    /// Products should always call [`crate::theme::install`] with their own
    /// palette before drawing.
    fn default() -> Self {
        Self {
            bg: Rgb::hex(0xf7_f8_fa),
            surface: Rgb::hex(0xff_ff_ff),
            surface_alt: Rgb::hex(0xee_f1_f5),
            text: Rgb::hex(0x1b_1f_26),
            text_muted: Rgb::hex(0x5d_66_75),
            accent: Rgb::hex(0x25_63_eb),
            accent_text: Rgb::hex(0xff_ff_ff),
            accent_soft: Rgb::hex(0xe9_ef_fc),
            border: Rgb::hex(0xdf_e4_ec),
            border_strong: Rgb::hex(0x7c_87_98),
            danger: Rgb::hex(0xd9_2d_20),
            success: Rgb::hex(0x12_80_5c),
            warning: Rgb::hex(0xb4_53_09),
        }
    }
}

impl Palette {
    /// Derive the accent family from one brand colour, keeping the product's
    /// own canvas, text, and semantic roles.
    ///
    /// `accent` is the fill colour; text drawn on it and text drawn in the
    /// accent colour are both derived by contrast, so a brand cannot make a
    /// button or a selected row unreadable. `dark` controls which way the soft
    /// wash is mixed: a dark surface needs a little more colour to be visible.
    pub fn with_accent(mut self, accent: Rgb, dark: bool) -> Self {
        self.accent = accent;
        self.accent_text = accent.best_ink();
        self.accent_soft = self.surface.mix(accent, if dark { 0.18 } else { 0.14 });
        self
    }

    /// The accent as text or a thin stroke.
    ///
    /// The brand fill itself is often too light for small text: amber on white,
    /// mint on white, and violet on a dark surface are the common failures. This
    /// walks the accent towards black or white only as far as needed to clear
    /// 4.5:1 against every surface it can land on, so the hue survives while the
    /// contrast does too. Use [`Palette::accent`] for fills; use this for labels,
    /// icons, links, and selected-row text.
    pub fn accent_ink(&self) -> Rgb {
        if self.readable(self.accent) {
            return self.accent;
        }

        let target = if self.bg.relative_luminance() < 0.5 {
            Rgb::WHITE
        } else {
            Rgb::BLACK
        };

        // Find the least adjustment that passes. Twenty steps is far more than
        // the eye can distinguish and keeps the result deterministic.
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        for _ in 0..20 {
            let middle = (low + high) * 0.5;
            if self.readable(self.accent.mix(target, middle)) {
                high = middle;
            } else {
                low = middle;
            }
        }
        self.accent.mix(target, high)
    }

    fn readable(&self, color: Rgb) -> bool {
        [self.bg, self.surface, self.surface_alt, self.accent_soft]
            .into_iter()
            .all(|background| color.contrast_ratio(background) >= 4.5)
    }
}

/// One built-in brand accent.
///
/// These are starting points, not a closed set: a product can derive the same
/// family from any [`Rgb`] with [`Palette::with_accent`]. The four presets avoid
/// the semantic roles already used by `warning`, `success`, and neutral chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brand {
    pub id: &'static str,
    pub name: &'static str,
    pub accent: Rgb,
}

pub const BRANDS: &[Brand] = &[
    Brand {
        id: "emerald",
        name: "Emerald",
        accent: Rgb::hex(0x04_78_57),
    },
    Brand {
        id: "ocean",
        name: "Ocean",
        accent: Rgb::hex(0x03_69_a1),
    },
    Brand {
        id: "violet",
        name: "Violet",
        accent: Rgb::hex(0x6d_28_d9),
    },
    Brand {
        id: "rose",
        name: "Rose",
        accent: Rgb::hex(0xbe_12_3c),
    },
];

/// Spacing scale. Everything in the UI should snap to one of these steps.
#[derive(Clone, Copy, Debug)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

pub const SPACING: Spacing = Spacing {
    xs: 4.0,
    sm: 8.0,
    md: 12.0,
    lg: 16.0,
    xl: 24.0,
};

/// Corner radii. Square corners read as "developer tool"; stay on the scale.
/// Deliberately restrained: a till is a tool, not a consumer app.
#[derive(Clone, Copy, Debug)]
pub struct Radii {
    /// Controls: buttons, inputs, navigation rows.
    pub sm: f32,
    /// Cards and tiles.
    pub md: f32,
    /// Dialogs, the cart column, the command palette.
    pub lg: f32,
    /// Fully rounded: badges, pills. Clamped by the backend at half the height.
    pub pill: f32,
}

pub const RADII: Radii = Radii {
    sm: 6.0,
    md: 10.0,
    lg: 14.0,
    pill: 999.0,
};

/// Type scale, in points. One step per job so a screen never invents a size.
///
/// `small` is the caption/status step, `body` the default, `button` the hit-area
/// label, `heading` a section title, `kpi` a dashboard figure, `total` the cart's
/// payable amount, and `receipt` the monospaced line printer preview.
#[derive(Clone, Copy, Debug)]
pub struct TypeScale {
    pub small: f32,
    /// Status bar and other persistent chrome: smaller than body, but a step
    /// above `small` so it stays readable at a glance from standing height.
    pub status: f32,
    pub body: f32,
    pub button: f32,
    pub heading: f32,
    pub kpi: f32,
    pub total: f32,
    pub receipt: f32,
}

pub const TYPE: TypeScale = TypeScale {
    small: 12.0,
    status: 13.5,
    body: 15.0,
    button: 15.0,
    heading: 20.0,
    kpi: 28.0,
    total: 32.0,
    receipt: 13.0,
};

/// Base font size multiplier. Left at 1.0 until a product actually needs a
/// denser or larger UI; per-app overrides are a feature to add deliberately.
pub const FONT_SCALE: f32 = 1.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// A neutral dark canvas for the derivation tests. `ui-kit` deliberately
    /// owns no product palettes, so the test brings its own.
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
        danger: Rgb::hex(0xf8_71_71),
        success: Rgb::hex(0x34_d3_99),
        warning: Rgb::hex(0xf5_9e_0b),
    };

    #[test]
    fn wcag_luminance_uses_linear_channels() {
        assert!((Rgb::WHITE.relative_luminance() - 1.0).abs() < 1e-6);
        assert!(Rgb::BLACK.relative_luminance().abs() < 1e-6);
        assert!((Rgb::WHITE.contrast_ratio(Rgb::BLACK) - 21.0).abs() < 1e-4);
        assert!((Rgb::hex(0x77_77_77).contrast_ratio(Rgb::WHITE) - 4.48).abs() < 0.02);
    }

    #[test]
    fn black_and_white_ink_are_chosen_by_contrast() {
        assert_eq!(Rgb::WHITE.best_ink(), Rgb::BLACK);
        assert_eq!(Rgb::BLACK.best_ink(), Rgb::WHITE);
        assert_eq!(Rgb::hex(0x04_78_57).best_ink(), Rgb::WHITE);
    }

    #[test]
    fn hex_round_trips_and_rejects_typos() {
        let color = Rgb::hex(0x04_78_57);
        assert_eq!(color.to_hex(), 0x04_78_57);
        assert_eq!(Rgb::parse_hex("#047857"), Some(color));
        assert_eq!(Rgb::parse_hex("047857"), Some(color));
        assert_eq!(Rgb::parse_hex("#04785"), None);
        assert_eq!(Rgb::parse_hex("#04785g"), None);
    }

    #[test]
    fn a_brand_accent_keeps_text_and_selected_rows_readable() {
        for brand in BRANDS {
            for (base, dark) in [(Palette::default(), false), (DARK, true)] {
                let palette = base.with_accent(brand.accent, dark);
                let ink = palette.accent_ink();

                assert!(
                    palette.accent_text.contrast_ratio(palette.accent) >= 4.49,
                    "{} text on fill in {} theme",
                    brand.id,
                    if dark { "dark" } else { "light" }
                );
                for background in [
                    palette.bg,
                    palette.surface,
                    palette.surface_alt,
                    palette.accent_soft,
                ] {
                    assert!(
                        ink.contrast_ratio(background) >= 4.49,
                        "{} accent ink on background in {} theme",
                        brand.id,
                        if dark { "dark" } else { "light" }
                    );
                }
                assert!(
                    palette.accent_soft.contrast_ratio(palette.surface) > 1.05,
                    "{} selected-row wash must be visible",
                    brand.id
                );
            }
        }
    }

    #[test]
    fn with_accent_leaves_every_non_accent_role_alone() {
        let base = Palette::default();
        let palette = base.with_accent(BRANDS[1].accent, false);

        assert_eq!(palette.bg, base.bg);
        assert_eq!(palette.surface, base.surface);
        assert_eq!(palette.text, base.text);
        assert_eq!(palette.danger, base.danger);
        assert_eq!(palette.success, base.success);
        assert_eq!(palette.warning, base.warning);
        assert_ne!(palette.accent, base.accent);
    }
}
