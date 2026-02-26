//! `taskcraft` binary entrypoint.
//!
//! The binary is intentionally thin and delegates command execution to the
//! library-backed CLI runtime in `taskcraft::run_cli`.

use anyhow::Result;

/// Starts the Taskcraft CLI process.
#[tokio::main]
async fn main() -> Result<()> {
    taskcraft::run_cli().await
}
