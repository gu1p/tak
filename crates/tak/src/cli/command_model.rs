use super::*;
use clap::CommandFactory;

mod remote;
mod task;

pub(super) use remote::{RemoteCommands, RemoteTaskCommands};
pub(super) use task::TaskCommands;

/// Tak task orchestrator.
#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(version = env!("TAK_VERSION"))]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    /// List every task from the current workspace with its label, dependencies, and description.
    List,
    /// Render the current workspace task graph as a tree.
    Tree,
    /// Print the Tak authoring bundle extracted from source comments and docstrings.
    Docs {
        #[command(subcommand)]
        command: DocsCommands,
    },
    /// Explain one task's resolved dependencies, steps, needs, timeout, and retry policy.
    Explain {
        /// The task label to explain.
        label: String,
    },
    /// Render the selected task graph in a machine-readable format.
    Graph {
        /// An optional root label that limits the rendered graph.
        label: Option<String>,
        /// The graph output format. Only `dot` is currently supported.
        #[arg(long, default_value = "dot")]
        format: String,
    },
    /// Serve the interactive task graph UI for the current workspace.
    Web {
        /// An optional root label that limits the rendered graph.
        label: Option<String>,
    },
    /// Execute an arbitrary command with Tak's runtime selection flags.
    Exec {
        /// Run the command from this working directory.
        #[arg(long = "cwd")]
        cwd: Option<String>,
        /// Inject one environment variable in `KEY=VALUE` form.
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
        /// Force local execution.
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        /// Force remote containerized execution.
        #[arg(long = "remote", default_value_t = false)]
        remote: bool,
        /// Force a local container. With `--remote`, accepted as a compatibility alias.
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        /// Use this container image for execution.
        #[arg(long = "container-image")]
        container_image: Option<String>,
        /// Build a container from this Dockerfile.
        #[arg(long = "container-dockerfile")]
        container_dockerfile: Option<String>,
        /// Override the Dockerfile build context directory.
        #[arg(long = "container-build-context")]
        container_build_context: Option<String>,
        /// The command and its arguments to execute.
        #[arg(last = true, required = true, num_args = 1.., allow_hyphen_values = true)]
        argv: Vec<String>,
    },
    /// Execute one or more task labels plus their dependencies.
    Run {
        /// The task labels to run. Bare task names resolve at the workspace root package.
        labels: Vec<String>,
        /// Maximum number of tasks to run in parallel.
        #[arg(short = 'j', long = "jobs", default_value_t = 1)]
        jobs: usize,
        /// Continue scheduling independent tasks after a task failure.
        #[arg(long = "keep-going", default_value_t = false)]
        keep_going: bool,
        /// Force local execution.
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        /// Force remote containerized execution.
        #[arg(long = "remote", default_value_t = false)]
        remote: bool,
        /// Force a local container. With `--remote`, accepted as a compatibility alias.
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        /// Use this container image for execution.
        #[arg(long = "container-image")]
        container_image: Option<String>,
        /// Build a container from this Dockerfile.
        #[arg(long = "container-dockerfile")]
        container_dockerfile: Option<String>,
        /// Override the Dockerfile build context directory.
        #[arg(long = "container-build-context")]
        container_build_context: Option<String>,
    },
    /// Manage remote execution agents configured on this machine.
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    /// Inspect task runs initiated by this local Tak client.
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
    /// Report coordination status when the current client build supports it.
    Status,
}

#[derive(Debug, Subcommand)]
pub(super) enum DocsCommands {
    /// Print the source-owned Tak authoring bundle for agents and contributors.
    Dump,
}

pub(crate) fn command_tree() -> clap::Command {
    Cli::command()
}
