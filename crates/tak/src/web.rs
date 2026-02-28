//! Embedded web graph visualization runtime for Tak.
//!
//! This module serves an interactive graph UI with fully embedded assets and opens a
//! browser tab in production builds.

include!("web/types_and_assets.rs");
include!("web/server.rs");
include!("web/payload.rs");
include!("web/handlers.rs");

#[cfg(test)]
mod tests;
