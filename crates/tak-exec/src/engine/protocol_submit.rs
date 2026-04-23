use std::time::Duration;

use prost::Message;
use tak_core::model::ResolvedTask;
use tak_proto::SubmitTaskResponse;

use super::{RemoteWorkspaceStage, StrictRemoteTarget};

use crate::remote_protocol_codec::build_remote_submit_payload;

use super::protocol_result_http::remote_protocol_http_request;
use super::remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind};

/// Submits one remote attempt after successful preflight.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn remote_protocol_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    _task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
) -> std::result::Result<(), RemoteSubmitFailure> {
    let body = build_remote_submit_payload(target, task_run_id, attempt, task, remote_workspace)
        .map_err(|err| RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!("{err:#}"),
        })?
        .encode_to_vec();
    let (status, response_body) = remote_protocol_http_request(
        target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        Duration::from_secs(1),
    )
    .await
    .map_err(|err| RemoteSubmitFailure {
        kind: RemoteSubmitFailureKind::Other,
        message: err.to_string(),
    })?;

    if status == 401 || status == 403 {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Auth,
            message: format!(
                "infra error: remote node {} auth failed during submit with HTTP {}",
                target.node_id, status
            ),
        });
    }
    if status != 200 {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} submit failed with HTTP {}",
                target.node_id, status
            ),
        });
    }

    let parsed =
        SubmitTaskResponse::decode(response_body.as_slice()).map_err(|_| RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} returned invalid protobuf for submit",
                target.node_id
            ),
        })?;
    if !parsed.accepted {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} rejected submit for task {} attempt {}",
                target.node_id, task.label, attempt
            ),
        });
    }
    if !parsed.remote_worker {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} returned submit acknowledgement without remote worker support",
                target.node_id
            ),
        });
    }

    Ok(())
}
