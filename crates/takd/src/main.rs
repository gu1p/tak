//! `takd` execution agent executable entrypoint.

use anyhow::Result;

mod cli;
mod logging;
mod qr_render;
mod serve_lock;
mod tor_secret_warning;
mod word_table;

#[tokio::main]
/// Starts the `takd` CLI.
async fn main() -> Result<()> {
    cli::run_cli().await
}
