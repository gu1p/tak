use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tak_proto::local_daemon::v2::{DaemonErrorCode, Request, Response};

use crate::cli::runs_cli::client::{RunDaemonClientError, send_response_with_timeout};

const FOREGROUND_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ATTEMPTS: usize = 3;

pub(super) async fn response(socket_path: &Path, request: &Request) -> Result<Response> {
    for attempt in 0..MAX_ATTEMPTS {
        match send_response_with_timeout(socket_path, request, FOREGROUND_TIMEOUT).await {
            Ok(Response::Error { code, .. }) => bail!(daemon_error(code)),
            Ok(response) => return Ok(response),
            Err(RunDaemonClientError::TimedOut | RunDaemonClientError::Disconnected)
                if attempt + 1 < MAX_ATTEMPTS =>
            {
                continue;
            }
            Err(error) => return Err(client_error(socket_path, error)),
        }
    }
    unreachable!("bounded exchange attempts always return")
}

fn daemon_error(code: DaemonErrorCode) -> &'static str {
    match code {
        DaemonErrorCode::ProtocolV2NotActive
        | DaemonErrorCode::ProtocolVersionInvalid
        | DaemonErrorCode::ProtocolVersionUnsupported => {
            "Protocol v2 mismatch; upgrade tak, takd, and workers together"
        }
        DaemonErrorCode::ProtocolRequestInvalid => "Local takd rejected the protocol v2 request",
        DaemonErrorCode::IdempotencyConflict => "Run submission idempotency conflict",
        DaemonErrorCode::RunNotFound => "The daemon-owned run was not found",
        DaemonErrorCode::WorkspaceInvalid => "Local takd rejected the workspace upload",
        DaemonErrorCode::RunStateInvalid => "The daemon-owned run is in an invalid state",
        DaemonErrorCode::RemoteInviteUnsupported => {
            "Remote invite is unsupported; upgrade tak, takd, and workers together"
        }
        DaemonErrorCode::Internal => "Local takd could not complete the run request",
    }
}

fn client_error(socket_path: &Path, error: RunDaemonClientError) -> anyhow::Error {
    match error {
        RunDaemonClientError::ConnectFailed => anyhow!(
            "Local takd is unavailable at {}; start `takd serve`; there is no client execution fallback.",
            socket_path.display()
        ),
        RunDaemonClientError::TimedOut | RunDaemonClientError::Disconnected => anyhow!(
            "Connection to local takd did not complete; submitted work is not cancelled. Reattach with `tak runs attach`."
        ),
        RunDaemonClientError::InvalidRequest
        | RunDaemonClientError::InvalidRunId
        | RunDaemonClientError::ProtocolMismatch => {
            anyhow!("Local takd protocol mismatch; upgrade tak, takd, and workers together")
        }
    }
}

pub(super) fn request_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}
