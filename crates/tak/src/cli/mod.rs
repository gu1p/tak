use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use tak_core::label::parse_label;
use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_loader::{LoadOptions, load_workspace};

mod command_model;
mod exec_cli;
mod graph_output;
mod remote_add;
mod remote_inventory;
mod remote_probe;
mod remote_probe_support;
mod remote_scan;
mod remote_status;
mod run_cli;
mod run_command;
mod run_output;
mod run_override_runtime;
#[cfg(test)]
mod run_override_runtime_tests;
mod run_overrides;
mod run_overrides_closure;
#[cfg(test)]
mod run_overrides_local_tests;
#[cfg(test)]
mod run_overrides_remote_tests;
mod run_overrides_support;
#[cfg(test)]
mod run_overrides_test_support;
mod workspace_helpers;

use command_model::{Cli, Commands};
use exec_cli::{ExecCliArgs, run_exec_command};
use graph_output::print_dot_graph;
use remote_add::{StartMode, run_remote_add};
use remote_inventory::{add_remote, list_remotes, remove_remote};
use remote_scan::run_remote_scan;
use remote_status::run_remote_status;
use run_command::{RunCliArgs, run_task_command};
use workspace_helpers::{canonical_label, load_workspace_from_cwd, parse_input_label};

pub(crate) use command_model::command_tree;
pub use run_cli::run_cli;
