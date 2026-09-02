//! Workspace discovery and `TASKS.py` loading.
//!
//! This crate discovers task definition files, evaluates them via Monty, converts output
//! into strongly-typed core models, and assembles a resolved `WorkspaceSpec`.

mod loader;

pub use loader::{
    LoadOptions, V2AuthoredRoot, detect_workspace_root, inspect_authored_root_module,
    load_workspace,
};

extern crate self as tak_loader;
