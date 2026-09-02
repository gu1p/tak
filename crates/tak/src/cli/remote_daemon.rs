use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tak_proto::local_daemon::v2::{DaemonErrorCode, Operation, Request, Response};

use super::runs_cli::client::{RunDaemonClientError, send_response, send_response_with_timeout};

const NETWORK_REMOTE_TIMEOUT: Duration = Duration::from_secs(300);

pub(in crate::cli) async fn request(operation: Operation, prefix: &str) -> Result<Response> {
    let socket = daemon_socket_path();
    let request = Request {
        request_id: format!("{prefix}-{}", uuid::Uuid::new_v4()),
        operation,
    };
    let response = match network_timeout(&request.operation) {
        Some(timeout) => send_response_with_timeout(&socket, &request, timeout).await,
        None => send_response(&socket, &request).await,
    };
    match response {
        Ok(Response::Error { code, .. }) => bail!(daemon_error(code)),
        Ok(response) => Ok(response),
        Err(error) => Err(client_error(&socket, error)),
    }
}

pub(super) fn network_timeout(operation: &Operation) -> Option<Duration> {
    matches!(
        operation,
        Operation::PreviewRemote { .. }
            | Operation::AddRemote { .. }
            | Operation::GetRemoteStatus { .. }
            | Operation::ReadRemote { .. }
    )
    .then_some(NETWORK_REMOTE_TIMEOUT)
}

pub(in crate::cli) fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}

fn daemon_error(code: DaemonErrorCode) -> &'static str {
    match code {
        DaemonErrorCode::ProtocolV2NotActive
        | DaemonErrorCode::ProtocolVersionInvalid
        | DaemonErrorCode::ProtocolVersionUnsupported => {
            "Local takd protocol mismatch; upgrade tak, takd, and workers together"
        }
        DaemonErrorCode::RemoteInviteUnsupported => {
            "Remote invite is unsupported; upgrade tak, takd, and workers together"
        }
        DaemonErrorCode::ProtocolRequestInvalid => {
            "Local takd rejected the protocol v2 remote request"
        }
        DaemonErrorCode::Internal => "Local takd could not complete the remote request",
        DaemonErrorCode::IdempotencyConflict
        | DaemonErrorCode::RunNotFound
        | DaemonErrorCode::WorkspaceInvalid
        | DaemonErrorCode::RunStateInvalid => {
            "Local takd returned an invalid response for a remote-management request"
        }
    }
}

fn client_error(socket: &Path, error: RunDaemonClientError) -> anyhow::Error {
    match error {
        RunDaemonClientError::ConnectFailed => anyhow!(
            "Local takd is required for remote management at {}; start `takd serve` or set TAKD_SOCKET. There is no client-side inventory or network fallback.",
            socket.display()
        ),
        RunDaemonClientError::TimedOut | RunDaemonClientError::Disconnected => anyhow!(
            "Local takd did not complete the remote-management request at {}; verify `takd serve` is healthy and retry.",
            socket.display()
        ),
        RunDaemonClientError::InvalidRequest
        | RunDaemonClientError::InvalidRunId
        | RunDaemonClientError::ProtocolMismatch => {
            anyhow!("Local takd protocol mismatch; upgrade tak, takd, and workers together")
        }
    }
}
