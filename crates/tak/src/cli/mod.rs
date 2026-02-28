use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use tak_core::label::parse_label;
use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_exec::{RunOptions, run_tasks};
use tak_loader::{LoadOptions, load_workspace};
use takd::{Request, Response, RunTasksRequest, StatusRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

mod command_model;
mod daemon_run_client;
mod daemon_status_client;
mod graph_output;
mod run_cli;
mod workspace_helpers;

use command_model::{Cli, Commands, DaemonCommands};
use daemon_run_client::try_run_via_daemon;
use daemon_status_client::query_daemon_status;
use graph_output::print_dot_graph;
use workspace_helpers::{
    env_u64, load_workspace_from_cwd, parse_input_label, resolve_daemon_socket_path,
};

pub use run_cli::run_cli;
