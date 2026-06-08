use super::*;
use clap::CommandFactory;

mod local;
mod remote;
mod task;

pub(super) use local::LocalCommands;
pub(super) use remote::{RemoteCommands, RemoteTaskCommands};
pub(super) use task::TaskCommands;

/// Tak task orchestrator.
#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(version = env!("TAK_VERSION"))]
pub(super) struct Cli {
    /// Force local execution for commands that support remote-by-default behavior.
    #[arg(long = "local", default_value_t = false)]
    pub(super) local: bool,
    /// Select a configured remote by alias, display name, node id, or node-id prefix.
    #[arg(long = "node")]
    pub(super) node: Option<String>,
    /// Require a remote architecture, for example `arm64` or `x86_64`.
    #[arg(long = "arch")]
    pub(super) arch: Option<String>,
    /// Require a remote operating system, for example `linux` or `macos`.
    #[arg(long = "os")]
    pub(super) os: Option<String>,
    /// Require a remote pool.
    #[arg(long = "pool")]
    pub(super) pool: Option<String>,
    /// Require one remote tag.
    #[arg(long = "tag")]
    pub(super) tags: Vec<String>,
    /// Require one remote capability.
    #[arg(long = "capability")]
    pub(super) capabilities: Vec<String>,
    /// Require a transport class: direct, tor, or any.
    #[arg(long = "transport", value_parser = ["direct", "tor", "any"])]
    pub(super) transport: Option<String>,
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
        /// Force local placement; a declared container runtime may still be used.
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        /// Force local host execution without a container.
        #[arg(long = "local-no-container", default_value_t = false)]
        local_no_container: bool,
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
        /// Force local placement; a declared container runtime may still be used.
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        /// Force local host execution without a container.
        #[arg(long = "local-no-container", default_value_t = false)]
        local_no_container: bool,
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
    /// Run Docker-shaped commands through Tak remote execution.
    Docker {
        /// Docker command tokens. `run` is supported; `build` is rejected with Tak guidance.
        #[arg(num_args = 1.., allow_hyphen_values = true, trailing_var_arg = true)]
        argv: Vec<String>,
    },
    /// Manage remote execution agents configured on this machine.
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    /// Inspect local execution status on this machine.
    Local {
        #[command(subcommand)]
        command: LocalCommands,
    },
    /// Inspect task runs initiated by this local Tak client.
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
    /// Show local and remote execution status.
    Status {
        /// Limit remote status output to these remote node ids.
        #[arg(long = "node")]
        node_ids: Vec<String>,
        /// Keep refreshing the status view until interrupted.
        #[arg(long, default_value_t = false)]
        watch: bool,
        /// Refresh the status snapshot every N milliseconds while watching.
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
    /// Update the installed tak and takd binaries from signed GitHub releases.
    Update {
        /// Only report whether a newer version exists; do not install.
        #[arg(long, default_value_t = false)]
        check: bool,
        /// Install even if it is the same or older version (allow downgrade).
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Install this exact version instead of the latest (e.g. 0.1.40 or v0.1.40).
        #[arg(long)]
        version: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum DocsCommands {
    /// Print the source-owned Tak authoring bundle for agents and contributors.
    Dump,
}

pub(crate) fn command_tree() -> clap::Command {
    Cli::command()
}
