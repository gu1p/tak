use super::*;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum LocalCommands {
    /// Show local task, container, resource, and daemon status.
    Status {
        /// Keep refreshing the status view until interrupted.
        #[arg(long, default_value_t = false)]
        watch: bool,
        /// Refresh the status snapshot every N milliseconds while watching.
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
}
