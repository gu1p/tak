use super::*;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RemoteCommands {
    /// Add one remote agent from a secret onboarding invite/token or Tor word phrase.
    Add {
        /// The secret onboarding invite/token emitted by `takd token show`.
        #[arg(conflicts_with = "words")]
        token: Option<String>,
        /// One or more secret onboarding words emitted by `takd token show --words`.
        #[arg(long = "words", value_name = "WORD", num_args = 0..)]
        words: Option<Vec<String>>,
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
    /// Print the service log from one configured remote node.
    Logs {
        /// The remote node id to inspect.
        #[arg(long = "node")]
        node_id: String,
        /// Print the complete remote service log.
        #[arg(long, default_value_t = false)]
        all: bool,
        /// Print the last N service log lines when `--all` is not set.
        #[arg(long, default_value_t = 200)]
        lines: usize,
    },
    /// List task attempts known by one configured remote node.
    Tasks {
        /// The remote node id to inspect.
        #[arg(long = "node")]
        node_id: String,
        /// Show only currently active task attempts.
        #[arg(long, default_value_t = false)]
        active: bool,
        /// Maximum number of task attempts to render.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Inspect task-centered data stored on one remote node.
    Task {
        #[command(subcommand)]
        command: RemoteTaskCommands,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RemoteTaskCommands {
    /// Print persisted stdout/stderr for one remote task run.
    Logs {
        /// The remote node id to inspect.
        #[arg(long = "node")]
        node_id: String,
        /// The task run id to inspect.
        task_run_id: String,
        /// Select a specific attempt for the task run.
        #[arg(long)]
        attempt: Option<u32>,
        /// Keep polling until the task reaches a terminal event.
        #[arg(long, default_value_t = false)]
        follow: bool,
        /// Poll interval in milliseconds while following.
        #[arg(long, default_value_t = 100)]
        interval_ms: u64,
    },
}
