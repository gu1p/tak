use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use tak_core::label::parse_label;
use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_exec::{RunOptions, run_tasks};
use tak_loader::{LoadOptions, load_workspace};

mod command_model;
mod graph_output;
mod remote_inventory;
mod remote_probe;
mod remote_probe_support;
mod remote_status;
mod run_cli;
mod workspace_helpers;

use command_model::{Cli, Commands};
use graph_output::print_dot_graph;
use remote_inventory::{add_remote, list_remotes, remove_remote};
use remote_status::run_remote_status;
use workspace_helpers::{canonical_label, load_workspace_from_cwd, parse_input_label};

pub use run_cli::run_cli;
