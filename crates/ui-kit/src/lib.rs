//! Shared look-and-feel for every zebra app.
//!
//! Presentation only: tokens, theme, and (behind the `widgets` and `icons`
//! features) widget primitives and the icon vocabulary. There is deliberately no
//! I/O, no domain vocabulary, and no process concern in this crate.
//!
//! The split between tokens and widgets is a feature, not a second crate: a
//! headless consumer can depend on `ui-kit` and get brand colours without
//! pulling in egui. If a second UI backend ever appears (web, PDF), that is the
//! point to split a `ui-egui` crate out — not before.

pub mod theme;
pub mod tokens;

#[cfg(feature = "widgets")]
pub mod components;

#[cfg(feature = "icons")]
pub mod icons;
