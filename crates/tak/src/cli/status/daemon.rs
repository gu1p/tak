use std::path::{Path, PathBuf};
use std::time::Duration;

use tak_proto::local_daemon::v2::{DaemonStatusSnapshot, Operation, Request, Response};
use tokio::time::timeout;

use crate::cli::runs_cli::client::send_response;

const DAEMON_STATUS_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub(super) enum LocalDaemonStatus {
    Available(LocalDaemonSnapshot),
    Unavailable { detail: String },
}

pub(super) type LocalDaemonSnapshot = DaemonStatusSnapshot;

pub(super) async fn local_daemon_status() -> LocalDaemonStatus {
    let socket_path = daemon_socket_path();

    match fetch_daemon_status(&socket_path).await {
        Ok(status) => LocalDaemonStatus::Available(status),
        Err(err) => LocalDaemonStatus::Unavailable {
            detail: err.to_string(),
        },
    }
}

fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}

async fn fetch_daemon_status(socket_path: &Path) -> anyhow::Result<LocalDaemonSnapshot> {
    match timeout(
        DAEMON_STATUS_TIMEOUT,
        fetch_daemon_status_inner(socket_path),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("daemon status request timed out"),
    }
}

async fn fetch_daemon_status_inner(socket_path: &Path) -> anyhow::Result<LocalDaemonSnapshot> {
    let request = Request {
        request_id: "tak-status".to_string(),
        operation: Operation::GetDaemonStatus {},
    };
    match send_response(socket_path, &request).await {
        Ok(Response::DaemonStatus { status, .. }) => Ok(status),
        Ok(Response::Error { .. }) => {
            anyhow::bail!("daemon rejected the protocol v2 status request")
        }
        Ok(_) => anyhow::bail!("unexpected protocol v2 daemon status response"),
        Err(error) => anyhow::bail!("protocol v2 daemon status request failed: {error:?}"),
    }
}
