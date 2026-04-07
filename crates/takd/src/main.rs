//! `takd` execution agent executable entrypoint.

use anyhow::Result;

mod cli;

#[tokio::main]
/// Starts the `takd` CLI.
async fn main() -> Result<()> {
    cli::run_cli().await
}
