//! Tak command-line interface for project-local `TASKS.py` workspaces.
//!
//! The CLI loads the current directory's `TASKS.py`, resolves the workspace graph,
//! and dispatches local, remote, and graph-inspection flows through one testable library.

#![recursion_limit = "256"]

extern crate self as tak;

mod cli;
mod diagnostics;
#[cfg(test)]
mod diagnostics_preflight_redaction_tests;
#[cfg(test)]
mod diagnostics_preflight_tests;
#[cfg(test)]
mod diagnostics_tests;
mod docs;
mod list_tui;
pub mod web;

pub use cli::run_cli;
pub use diagnostics::render_error_report;
