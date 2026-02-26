//! Core domain primitives for Taskcraft.
//!
//! This crate defines shared model types, label parsing, and DAG planning logic that
//! are reused by loader, executor, daemon, and CLI crates.

pub mod label;
pub mod model;
pub mod planner;
