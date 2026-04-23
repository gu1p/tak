use super::*;
use clap::CommandFactory;

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
        /// Force a local container runtime. With `--remote`, accepted as a compatibility alias.
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        /// Use this container image for execution.
        #[arg(long = "container-image")]
        container_image: Option<String>,
        /// Build a container runtime from this Dockerfile.
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
        /// Force a local container runtime. With `--remote`, accepted as a compatibility alias.
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        /// Use this container image for execution.
        #[arg(long = "container-image")]
        container_image: Option<String>,
        /// Build a container runtime from this Dockerfile.
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
    /// Report coordination status when the current client build supports it.
    Status,
}

#[derive(Debug, Subcommand)]
pub(super) enum DocsCommands {
    /// Print the source-owned Tak authoring bundle for agents and contributors.
    Dump,
}

#[derive(Debug, Subcommand)]
pub(super) enum RemoteCommands {
    /// Add one remote agent from an onboarding token or Tor word phrase.
    Add {
        /// The onboarding token emitted by `takd token show`.
        #[arg(required_unless_present = "words", conflicts_with = "words")]
        token: Option<String>,
        /// One or more onboarding words emitted by `takd token show --words`.
        #[arg(
            long = "words",
            value_name = "WORD",
            num_args = 1..,
            required_unless_present = "token"
        )]
        words: Vec<String>,
    },
    /// Scan a QR code and add the discovered remote agent.
    Scan,
    /// List configured remote agents in client priority order.
    List,
    /// Remove one configured remote agent by node id.
    Remove {
        /// The remote node id to remove.
        node_id: String,
    },
    /// Show the current status for configured remote agents.
    Status {
        /// Limit status output to these remote node ids.
        #[arg(long = "node")]
        node_ids: Vec<String>,
        /// Keep refreshing the status view until interrupted.
        #[arg(long, default_value_t = false)]
        watch: bool,
        /// Refresh the node snapshot every N milliseconds while watching.
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
}

pub(crate) fn command_tree() -> clap::Command {
    Cli::command()
}
