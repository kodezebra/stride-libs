//! The icon vocabulary, behind the `icons` feature.
//!
//! Apps name a glyph through [`Icon`] and never through the icon crate, so
//! replacing the set (or vendoring a smaller one for a specific product) is a
//! change in this file alone. The same rule as [`crate::theme`]: one crate owns
//! the visual contract, screens only ask for a name.
//!
//! Icons are SVG, so [`crate::theme::install`] also registers the image loaders
//! when this feature is on. Nothing else needs to call
//! `egui_extras::install_image_loaders`.

/// One glyph, drawn as a stroked SVG at the size the caller asks for.
///
/// `Copy` and tiny, so it can live in a `const` table next to the thing it
/// labels — see `ui-shell`'s navigation rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Icon(egui_lucide::Lucide);

impl Icon {
    /// Stroke width in the 24-unit viewBox. Lucide's own default is 2.0;
    /// 1.75 reads better at the 18–22 px sizes used in chrome.
    pub const STROKE: f32 = 1.75;

    /// Look an icon up by its Lucide kebab-case name, e.g. `"shopping-cart"`.
    ///
    /// Used when an icon comes from a settings file, where the glyph is a
    /// string and an unknown value must not panic.
    pub fn named(name: &str) -> Option<Self> {
        egui_lucide::Lucide::from_name(name).map(Self)
    }

    /// Kebab-case name, the inverse of [`Icon::named`].
    pub fn name(self) -> &'static str {
        self.0.name()
    }

    /// Draw the icon at `size` points in the current text colour.
    pub fn paint(self, ui: &mut egui::Ui, size: f32) -> egui::Response {
        ui.add(
            egui_lucide::LucideIcon::new(self.0)
                .size(size)
                .stroke_width(Self::STROKE),
        )
    }

    /// Draw the icon at `size` points in an explicit colour, for glyphs that
    /// carry state (offline, queued, failed).
    pub fn paint_colored(
        self,
        ui: &mut egui::Ui,
        size: f32,
        color: egui::Color32,
    ) -> egui::Response {
        ui.add(
            egui_lucide::LucideIcon::new(self.0)
                .size(size)
                .stroke_width(Self::STROKE)
                .color(color),
        )
    }

    /// The icon as an [`egui::Image`], for callers that place it themselves.
    ///
    /// Share [`Icon::STROKE`] with [`Icon::paint`] so a hand-placed glyph has the
    /// same weight as the ones drawn by the shared widgets.
    pub fn image(self, size: f32, color: egui::Color32) -> egui::Image<'static> {
        egui_lucide::LucideIcon::new(self.0)
            .size(size)
            .stroke_width(Self::STROKE)
            .color(color)
            .image()
    }
}

/// The glyphs the products actually use.
///
/// Named after the drawing, not the business meaning, because `ui-kit` carries
/// no domain vocabulary. The doc comment is where the intended role lives.
///
/// Adding an icon means adding a constant here — that is the point. An app that
/// reaches past this list is asking for a vocabulary decision, not a one-liner.
impl Icon {
    /// Shell: generic home.
    pub const HOUSE: Self = Self(egui_lucide::Lucide::House);
    /// Navigation: dashboard / overview.
    pub const DASHBOARD: Self = Self(egui_lucide::Lucide::LayoutDashboard);
    /// Navigation: selling / till.
    pub const CART: Self = Self(egui_lucide::Lucide::ShoppingCart);
    /// Navigation: products / catalogue.
    pub const PACKAGE: Self = Self(egui_lucide::Lucide::Package);
    /// Navigation: sales history, receipts.
    pub const RECEIPT: Self = Self(egui_lucide::Lucide::Receipt);
    /// Navigation: settings.
    pub const SETTINGS: Self = Self(egui_lucide::Lucide::Settings);
    /// Navigation: stock, future inventory module.
    pub const BOXES: Self = Self(egui_lucide::Lucide::Boxes);
    /// Navigation: suppliers, future purchasing module.
    pub const TRUCK: Self = Self(egui_lucide::Lucide::Truck);

    /// Action: search / scan field.
    pub const SEARCH: Self = Self(egui_lucide::Lucide::Search);
    /// Action: barcode scan.
    pub const SCAN: Self = Self(egui_lucide::Lucide::ScanBarcode);
    /// Action: add.
    pub const PLUS: Self = Self(egui_lucide::Lucide::Plus);
    /// Action: remove one.
    pub const MINUS: Self = Self(egui_lucide::Lucide::Minus);
    /// Action: dismiss / clear line.
    pub const CLOSE: Self = Self(egui_lucide::Lucide::X);
    /// Action: delete / void.
    pub const TRASH: Self = Self(egui_lucide::Lucide::Trash2);
    /// Action: confirm.
    pub const CHECK: Self = Self(egui_lucide::Lucide::Check);
    /// Action: print a receipt.
    pub const PRINT: Self = Self(egui_lucide::Lucide::Printer);
    /// Action: refund / reverse.
    pub const REFUND: Self = Self(egui_lucide::Lucide::RotateCcw);
    /// Action: back.
    pub const BACK: Self = Self(egui_lucide::Lucide::ArrowLeft);

    /// Direction: collapsed / expandable.
    pub const CHEVRON_DOWN: Self = Self(egui_lucide::Lucide::ChevronDown);
    /// Direction: previous.
    pub const CHEVRON_LEFT: Self = Self(egui_lucide::Lucide::ChevronLeft);
    /// Direction: next.
    pub const CHEVRON_RIGHT: Self = Self(egui_lucide::Lucide::ChevronRight);

    /// Status: online.
    pub const ONLINE: Self = Self(egui_lucide::Lucide::Wifi);
    /// Status: offline, still usable.
    pub const OFFLINE: Self = Self(egui_lucide::Lucide::WifiOff);
    /// Status: not synced (cloud unreachable).
    pub const UNSYNCED: Self = Self(egui_lucide::Lucide::CloudOff);
    /// Status: warning.
    pub const WARNING: Self = Self(egui_lucide::Lucide::CircleAlert);
    /// Status: done.
    pub const DONE: Self = Self(egui_lucide::Lucide::CircleCheck);
    /// Status: busy.
    pub const BUSY: Self = Self(egui_lucide::Lucide::Loader);

    /// Identity: cashier / current user.
    pub const USER: Self = Self(egui_lucide::Lucide::User);
    /// Identity: terminal / device.
    pub const TERMINAL: Self = Self(egui_lucide::Lucide::CreditCard);
    /// Identity: money.
    pub const MONEY: Self = Self(egui_lucide::Lucide::Banknote);
    /// Identity: document.
    pub const DOCUMENT: Self = Self(egui_lucide::Lucide::FileText);

    /// Chrome: sidebar toggle.
    pub const SIDEBAR: Self = Self(egui_lucide::Lucide::PanelLeft);
    /// Chrome: command palette.
    pub const PALETTE: Self = Self(egui_lucide::Lucide::Command);
    /// Appearance: light theme.
    pub const SUN: Self = Self(egui_lucide::Lucide::Sun);
    /// Appearance: dark theme.
    pub const MOON: Self = Self(egui_lucide::Lucide::Moon);
}
