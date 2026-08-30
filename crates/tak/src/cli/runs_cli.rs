use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{DaemonErrorCode, Operation, Request};

use super::command_model::RunsCommands;

#[path = "runs_cli/client.rs"]
mod client;
#[cfg(test)]
mod client_deadline_tests;
use client::RunDaemonClientError;

const INACTIVE_DIAGNOSTIC: &str = "Protocol v2 run operations are not active in this takd build; upgrade tak, takd, and workers together.";
const ADMISSION_REJECTION_DIAGNOSTIC: &str = "Local takd rejected the protocol v2 request before dispatch, so the requested operation was not persisted; upgrade tak, takd, and workers together; no legacy fallback was attempted.";
const MISMATCH_DIAGNOSTIC: &str = "Local takd protocol mismatch; upgrade tak, takd, and workers together. No legacy fallback was attempted.";
const UNKNOWN_OUTCOME_DIAGNOSTIC: &str =
    "The request outcome is unknown. A failed client exchange is not a cancellation signal.";
const CANCELLATION_OUTCOME_DIAGNOSTIC: &str = "A cancellation request may already be persisted.";
const RECOVERY_GUIDANCE: &str = "Check state with `tak runs show RUN_ID`; use `tak runs list` to find a run or `tak runs attach RUN_ID` to reconnect; there is no client execution fallback.";

pub(super) async fn run_runs_command(command: RunsCommands) -> Result<ExitCode> {
    let cancellation_requested = matches!(&command, RunsCommands::Cancel { .. });
    let request = request_for(command);
    let socket_path = daemon_socket_path();
    match client::send_request(&socket_path, &request).await {
        Ok(DaemonErrorCode::ProtocolV2NotActive) => bail!(INACTIVE_DIAGNOSTIC),
        Ok(
            DaemonErrorCode::ProtocolVersionInvalid
            | DaemonErrorCode::ProtocolVersionUnsupported
            | DaemonErrorCode::ProtocolRequestInvalid,
        ) => bail!(ADMISSION_REJECTION_DIAGNOSTIC),
        Err(RunDaemonClientError::InvalidRequest) => {
            bail!("Protocol v2 run request is invalid.")
        }
        Err(RunDaemonClientError::InvalidRunId) => {
            bail!("Run ID is invalid; expected 1 to 128 UTF-8 bytes with no control characters.")
        }
        Err(RunDaemonClientError::ProtocolMismatch) => {
            bail!(
                "{MISMATCH_DIAGNOSTIC} {}",
                unknown_outcome_guidance(cancellation_requested)
            )
        }
        Err(RunDaemonClientError::TimedOut) => bail!(
            "Local takd at {} did not provide a complete response before the client timeout. {}",
            socket_path.display(),
            unknown_outcome_guidance(cancellation_requested),
        ),
        Err(RunDaemonClientError::ConnectFailed) => bail!(
            "Local takd is unavailable at {}; start `takd serve`; there is no client execution fallback.",
            socket_path.display()
        ),
        Err(RunDaemonClientError::Disconnected) => bail!(
            "Connection to local takd at {} closed before a complete response. {}",
            socket_path.display(),
            unknown_outcome_guidance(cancellation_requested),
        ),
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

fn request_for(command: RunsCommands) -> Request {
    let (request_id, operation) = match command {
        RunsCommands::List => ("tak-runs-list", Operation::ListRuns {}),
        RunsCommands::Show { run_id } => ("tak-runs-show", Operation::GetRun { run_id }),
        RunsCommands::Attach { run_id } => (
            "tak-runs-attach",
            Operation::AttachRun {
                run_id,
                after_event: 0,
            },
        ),
        RunsCommands::Cancel { run_id } => ("tak-runs-cancel", Operation::CancelRun { run_id }),
        RunsCommands::Outputs { run_id, to: _ } => {
            ("tak-runs-outputs", Operation::GetOutputManifest { run_id })
        }
    };
    Request {
        request_id: request_id.into(),
        operation,
    }
}

fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}
