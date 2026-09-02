use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{DaemonErrorCode, Operation, Request, Response};

use super::command_model::RunsCommands;

#[path = "runs_cli/attach.rs"]
pub(crate) mod attach;
#[path = "runs_cli/client.rs"]
pub(crate) mod client;
#[cfg(test)]
mod client_deadline_tests;
#[path = "runs_cli/outputs.rs"]
pub(crate) mod outputs;
#[path = "runs_cli/render.rs"]
mod render;

use client::RunDaemonClientError;

const INACTIVE_DIAGNOSTIC: &str = "Protocol v2 run operations are not active in this takd build; upgrade tak, takd, and workers together.";
const ADMISSION_REJECTION_DIAGNOSTIC: &str = "Local takd rejected the protocol v2 request before dispatch, so the requested operation was not persisted; upgrade tak, takd, and workers together; no legacy fallback was attempted.";
const MISMATCH_DIAGNOSTIC: &str = "Local takd protocol mismatch; upgrade tak, takd, and workers together. No legacy fallback was attempted.";
const UNKNOWN_OUTCOME_DIAGNOSTIC: &str =
    "The request outcome is unknown. A failed client exchange is not a cancellation signal.";
const CANCELLATION_OUTCOME_DIAGNOSTIC: &str = "A cancellation request may already be persisted.";
const RECOVERY_GUIDANCE: &str = "Check state with `tak runs show RUN_ID`; use `tak runs list` to find a run or `tak runs attach RUN_ID` to reconnect; there is no client execution fallback.";

pub(super) async fn run_runs_command(command: RunsCommands) -> Result<ExitCode> {
    let socket = daemon_socket_path();
    match command {
        RunsCommands::List => {
            let response = request(&socket, "tak-runs-list", Operation::ListRuns {}, false).await?;
            let Response::RunList { runs, .. } = response else {
                bail!(MISMATCH_DIAGNOSTIC)
            };
            render::list(&runs);
        }
        RunsCommands::Show { run_id } => {
            let response = request(
                &socket,
                "tak-runs-show",
                Operation::GetRun {
                    run_id: run_id.clone(),
                },
                false,
            )
            .await?;
            let Response::RunDetails { run, .. } = response else {
                bail!(MISMATCH_DIAGNOSTIC)
            };
            if run.summary.run_id != run_id {
                bail!(MISMATCH_DIAGNOSTIC);
            }
            render::details(&run);
        }
        RunsCommands::Attach { run_id } => return attach::run(&socket, &run_id).await,
        RunsCommands::Cancel { run_id } => {
            let response = request(
                &socket,
                "tak-runs-cancel",
                Operation::CancelRun {
                    run_id: run_id.clone(),
                },
                true,
            )
            .await?;
            let Response::CancellationAccepted {
                run_id: response_run,
                state,
                ..
            } = response
            else {
                bail!(MISMATCH_DIAGNOSTIC)
            };
            if response_run != run_id {
                bail!(MISMATCH_DIAGNOSTIC);
            }
            println!("{run_id} {}", state.as_str());
        }
        RunsCommands::Outputs { run_id, to } => outputs::retrieve(&socket, &run_id, &to).await?,
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) async fn request(
    socket: &Path,
    request_id: &str,
    operation: Operation,
    cancellation_requested: bool,
) -> Result<Response> {
    let request = Request {
        request_id: request_id.into(),
        operation,
    };
    match client::send_response(socket, &request).await {
        Ok(Response::Error { code, .. }) => daemon_error(code),
        Ok(response) => Ok(response),
        Err(RunDaemonClientError::InvalidRequest) => bail!("Protocol v2 run request is invalid."),
        Err(RunDaemonClientError::InvalidRunId) => {
            bail!("Run ID is invalid; expected 1 to 128 UTF-8 bytes with no control characters.")
        }
        Err(RunDaemonClientError::ProtocolMismatch) => bail!(
            "{MISMATCH_DIAGNOSTIC} {}",
            unknown_outcome_guidance(cancellation_requested)
        ),
        Err(RunDaemonClientError::TimedOut) => bail!(
            "Local takd at {} did not provide a complete response before the client timeout. {}",
            socket.display(),
            unknown_outcome_guidance(cancellation_requested),
        ),
        Err(RunDaemonClientError::ConnectFailed) => bail!(
            "Local takd is unavailable at {}; start `takd serve`; there is no client execution fallback.",
            socket.display()
        ),
        Err(RunDaemonClientError::Disconnected) => bail!(
            "Connection to local takd at {} closed before a complete response. {}",
            socket.display(),
            unknown_outcome_guidance(cancellation_requested),
        ),
    }
}

fn daemon_error(code: DaemonErrorCode) -> Result<Response> {
    match code {
        DaemonErrorCode::ProtocolV2NotActive => bail!(INACTIVE_DIAGNOSTIC),
        DaemonErrorCode::ProtocolVersionInvalid
        | DaemonErrorCode::ProtocolVersionUnsupported
        | DaemonErrorCode::ProtocolRequestInvalid => bail!(ADMISSION_REJECTION_DIAGNOSTIC),
        DaemonErrorCode::IdempotencyConflict => bail!("Run submission idempotency conflict."),
        DaemonErrorCode::RunNotFound => bail!("Daemon-owned run not found."),
        DaemonErrorCode::WorkspaceInvalid => bail!("Daemon rejected the workspace payload."),
        DaemonErrorCode::RunStateInvalid => bail!("Run operation is invalid in its current state."),
        DaemonErrorCode::RemoteInviteUnsupported => {
            bail!("Remote invite is unsupported; upgrade tak, takd, and workers together.")
        }
        DaemonErrorCode::Internal => bail!("Local takd could not complete the run operation."),
    }
}

fn unknown_outcome_guidance(cancellation_requested: bool) -> String {
    if cancellation_requested {
        return format!(
            "{UNKNOWN_OUTCOME_DIAGNOSTIC} {CANCELLATION_OUTCOME_DIAGNOSTIC} {RECOVERY_GUIDANCE}"
        );
    }
    format!("{UNKNOWN_OUTCOME_DIAGNOSTIC} {RECOVERY_GUIDANCE}")
}

fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}
