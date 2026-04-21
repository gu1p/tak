//! Workspace discovery and `TASKS.py` loading.
//!
//! This crate discovers task definition files, evaluates them via Monty, converts output
//! into strongly-typed core models, and assembles a resolved `WorkspaceSpec`.

mod loader;

pub use loader::{
    LoadOptions, detect_workspace_root, discover_tasks_files, evaluate_named_policy_decision,
    load_workspace,
};

extern crate self as tak_loader;
