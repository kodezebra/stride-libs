//! Panic handling.
//!
//! A GUI app launched from a desktop shortcut has no terminal, so a bare panic
//! is a window that vanishes with no explanation. This hook logs the panic
//! (into the same file as everything else) and prints a pointer to that file
//! before chaining to the default hook, which still prints the backtrace.
//!
//! A native "something went wrong" dialog belongs here eventually, behind an
//! optional feature with `rfd` as the dependency. `rfd` is not in the local
//! cache, so that is deliberately left out until it is actually wanted.

use std::panic;

/// Install the hook. Called by [`crate::init`]; not meant to be called twice.
pub fn install(app: &'static str) {
    let previous = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "panic", "{info}");
        eprintln!("{app} stopped unexpectedly. Details are in the log file.");
        previous(info);
    }));
}
