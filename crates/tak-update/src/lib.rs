//! Self-update engine for the `tak` CLI and `takd` daemon.
//!
//! This crate resolves the latest published GitHub release, downloads and verifies
//! the release archive (signature + checksum), and atomically swaps the running
//! binaries on disk. Behavior is expressed through small ports (traits) so the
//! `tak`/`takd` commands and the daemon background loop reuse one use-case with
//! test fakes instead of touching the network or the filesystem.
//!
//! Modules are added incrementally as the feature lands; see the crate's plan for
//! the staged layout.

pub mod archive;
pub mod fs_installer;
pub mod http;
pub mod install_target;
pub mod installer;
pub mod plan;
pub mod release_client;
pub mod runner;
pub mod swap;
pub mod target;
pub mod validate;
pub mod verify;
pub mod version;
