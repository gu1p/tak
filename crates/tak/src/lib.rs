//! Tak command-line interface library.
//!
//! This crate exposes the CLI runtime used by the `tak` binary. Moving behavior
//! into the library keeps command logic testable and doctestable.

mod cli;
mod list_tui;
pub mod web;

pub use cli::run_cli;
