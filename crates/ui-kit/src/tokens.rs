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
