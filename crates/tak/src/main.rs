//! `tak` binary entrypoint.
//!
//! The binary is intentionally thin and delegates command execution to the
//! library-backed CLI runtime in `tak::run_cli`.

use anyhow::Result;

/// Starts the Tak CLI process.
#[tokio::main]
async fn main() -> Result<()> {
    tak::run_cli().await
}
