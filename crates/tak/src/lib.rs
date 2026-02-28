//! Tak command-line interface library.
//!
//! This crate exposes the CLI runtime used by the `tak` binary. Moving behavior
//! into the library keeps command logic testable and doctestable.

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

mod list_tui;
pub mod web;

include!("cli/command_model.rs");
include!("cli/run_cli.rs");
include!("cli/workspace_helpers.rs");
include!("cli/daemon_run_client.rs");
include!("cli/graph_output.rs");
include!("cli/daemon_status_client.rs");
