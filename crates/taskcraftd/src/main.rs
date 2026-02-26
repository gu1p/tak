//! `taskcraftd` daemon executable entrypoint.

use anyhow::Result;

#[tokio::main]
/// Starts the daemon using the default socket path and runtime configuration.
async fn main() -> Result<()> {
    let socket = taskcraftd::default_socket_path();
    taskcraftd::run_daemon(&socket).await
}
