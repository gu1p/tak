use clap::Parser;

use super::Commands;

/// Tak task orchestrator.
#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(version = env!("TAK_VERSION"))]
pub(in crate::cli) struct Cli {
    /// Force local execution for commands that support remote-by-default behavior.
    #[arg(long = "local", default_value_t = false)]
    pub(in crate::cli) local: bool,
    /// Select a configured remote by alias, display name, node id, or node-id prefix.
    #[arg(long = "node")]
    pub(in crate::cli) node: Option<String>,
    /// Require a remote architecture, for example `arm64` or `x86_64`.
    #[arg(long = "arch")]
    pub(in crate::cli) arch: Option<String>,
    /// Require a remote operating system, for example `linux` or `macos`.
    #[arg(long = "os")]
    pub(in crate::cli) os: Option<String>,
    /// Require a remote pool.
    #[arg(long = "pool")]
    pub(in crate::cli) pool: Option<String>,
    /// Require one remote tag.
    #[arg(long = "tag")]
    pub(in crate::cli) tags: Vec<String>,
    /// Require one remote capability.
    #[arg(long = "capability")]
    pub(in crate::cli) capabilities: Vec<String>,
    /// Require a transport class: direct, tor, or any.
    #[arg(long = "transport", value_parser = ["direct", "tor", "any"])]
    pub(in crate::cli) transport: Option<String>,
    #[command(subcommand)]
    pub(in crate::cli) command: Commands,
}
