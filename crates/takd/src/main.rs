//! `takd` daemon executable entrypoint.

use anyhow::Result;

#[tokio::main]
/// Starts the daemon using the default socket path and runtime configuration.
async fn main() -> Result<()> {
    let socket = takd::default_socket_path();
    takd::run_daemon(&socket).await
}
