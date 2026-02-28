//! Embedded web graph visualization runtime for Tak.
//!
//! This module serves an interactive graph UI with fully embedded assets and opens a
//! browser tab in production builds.

mod handlers;
mod payload;
mod server;
mod types_and_assets;

pub use server::serve_graph_ui;

#[cfg(test)]
mod tests;
