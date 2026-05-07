use super::*;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum TaskCommands {
    /// List task runs initiated by this local Tak client.
    List {
        /// Maximum number of local task runs to render.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Print captured stdout/stderr for one local task run.
    Logs {
        /// The local task run id to inspect.
        task_run_id: String,
        /// Keep polling until the task reaches a terminal state.
        #[arg(long, default_value_t = false)]
        follow: bool,
        /// Poll interval in milliseconds while following.
        #[arg(long, default_value_t = 100)]
        interval_ms: u64,
    },
}
