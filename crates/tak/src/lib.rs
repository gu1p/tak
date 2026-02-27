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
use takd::{Request, Response, StatusRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

mod list_tui;
pub mod web;

#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(about = "Tak task orchestrator")]
#[command(version = env!("TAK_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    List,
    Tree,
    Explain {
        label: String,
    },
    Graph {
        label: Option<String>,
        #[arg(long, default_value = "dot")]
        format: String,
    },
    Web {
        label: Option<String>,
    },
    Run {
        labels: Vec<String>,
        #[arg(short = 'j', long = "jobs", default_value_t = 1)]
        jobs: usize,
        #[arg(long = "keep-going", default_value_t = false)]
        keep_going: bool,
    },
    Status,
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommands {
    Start,
    Status,
}

/// Parses CLI input and dispatches Tak commands.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", list_tui::render_list(&spec));
        }
        Commands::Tree => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", list_tui::render_tree(&spec)?);
        }
        Commands::Explain { label } => {
            let spec = load_workspace_from_cwd()?;
            let label = parse_input_label(&label)?;
            let task = spec
                .tasks
                .get(&label)
                .ok_or_else(|| anyhow!("task not found: {label}"))?;

            println!("label: {label}");
            if task.deps.is_empty() {
                println!("deps: (none)");
            } else {
                println!("deps:");
                for dep in &task.deps {
                    println!("  - {dep}");
                }
            }
            println!("steps: {}", task.steps.len());
            println!("needs: {}", task.needs.len());
            println!(
                "timeout_s: {}",
                task.timeout_s
                    .map_or_else(|| "none".to_string(), |s| s.to_string())
            );
            println!("retry_attempts: {}", task.retry.attempts);
        }
        Commands::Graph { label, format } => {
            let spec = load_workspace_from_cwd()?;
            if format != "dot" {
                bail!("unsupported format: {format}");
            }
            let scope = match label {
                Some(value) => vec![parse_input_label(&value)?],
                None => spec.tasks.keys().cloned().collect(),
            };
            print_dot_graph(&spec, &scope);
        }
        Commands::Web { label } => {
            let spec = load_workspace_from_cwd()?;
            let parsed = label
                .as_deref()
                .map(parse_input_label)
                .transpose()
                .context("failed to parse optional web graph label")?;
            web::serve_graph_ui(&spec, parsed.as_ref()).await?;
        }
        Commands::Run {
            labels,
            jobs,
            keep_going,
        } => {
            if labels.is_empty() {
                bail!("run requires at least one label");
            }

            let spec = load_workspace_from_cwd()?;
            let targets = labels
                .iter()
                .map(|label| parse_input_label(label))
                .collect::<Result<Vec<_>>>()?;

            let summary = run_tasks(
                &spec,
                &targets,
                &RunOptions {
                    jobs,
                    keep_going,
                    lease_socket: Some(resolve_daemon_socket_path()),
                    lease_ttl_ms: env_u64("TAK_LEASE_TTL_MS", 30_000),
                    lease_poll_interval_ms: env_u64("TAK_LEASE_POLL_MS", 200),
                    session_id: std::env::var("TAK_SESSION_ID").ok(),
                    user: std::env::var("TAK_USER").ok(),
                },
            )
            .await?;

            for (label, result) in summary.results {
                println!(
                    "{label}: {} (attempts={}, exit_code={})",
                    if result.success { "ok" } else { "failed" },
                    result.attempts,
                    result
                        .exit_code
                        .map_or_else(|| "none".to_string(), |code| code.to_string())
                );
            }
        }
        Commands::Status => {
            let snapshot = query_daemon_status(resolve_daemon_socket_path()).await?;
            println!("active_leases: {}", snapshot.active_leases);
            println!("pending_requests: {}", snapshot.pending_requests);
            for usage in snapshot.usage {
                println!(
                    "usage {} {:?} {:?}: {}/{}",
                    usage.name, usage.scope, usage.scope_key, usage.used, usage.capacity
                );
            }
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Start => {
                let socket = resolve_daemon_socket_path();
                takd::run_daemon(&socket).await?;
            }
            DaemonCommands::Status => {
                let snapshot = query_daemon_status(resolve_daemon_socket_path()).await?;
                println!("active_leases: {}", snapshot.active_leases);
                println!("pending_requests: {}", snapshot.pending_requests);
            }
        },
    }

    Ok(())
}

/// Loads a workspace from the current working directory using default loader options.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn load_workspace_from_cwd() -> Result<WorkspaceSpec> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    load_workspace(&cwd, &LoadOptions::default())
}

/// Parses a user-provided CLI label into a fully validated `TaskLabel`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn parse_input_label(value: &str) -> Result<TaskLabel> {
    parse_label(value, "//").map_err(|e| anyhow!("invalid label {value}: {e}"))
}

/// Resolves daemon socket path from environment override or default path logic.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_daemon_socket_path() -> PathBuf {
    std::env::var("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| takd::default_socket_path())
}

/// Reads a `u64` value from an environment variable with a fallback default.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn env_u64(var_name: &str, default: u64) -> u64 {
    std::env::var(var_name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

/// Renders a DOT graph for the selected task scope.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn print_dot_graph(spec: &WorkspaceSpec, scope: &[TaskLabel]) {
    println!("digraph tak {{");
    for label in scope {
        if let Some(task) = spec.tasks.get(label) {
            if task.deps.is_empty() {
                println!("  \"{label}\";");
            } else {
                for dep in &task.deps {
                    println!("  \"{dep}\" -> \"{label}\";");
                }
            }
        }
    }
    println!("}}");
}

/// Requests daemon status over the Unix socket protocol.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn query_daemon_status(socket_path: PathBuf) -> Result<takd::StatusSnapshot> {
    let stream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let request = Request::Status(StatusRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before responding");
    }

    match serde_json::from_str::<Response>(line.trim_end())? {
        Response::StatusSnapshot { status, .. } => Ok(status),
        Response::Error { message, .. } => bail!("daemon error: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}
