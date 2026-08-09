//! Tak command-line interface for project-local task workspaces and Makefiles.
//!
//! Graph commands load the current directory's `TASKS.py`; `tak make` instead resolves one
//! annotated Makefile goal. Both paths dispatch local or remote execution through the same
//! testable runtime boundary.

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
mod remote_alias;
pub mod web;

pub use cli::run_cli;
pub use diagnostics::render_error_report;
pub use remote_alias::remote_alias_for_node_id;
