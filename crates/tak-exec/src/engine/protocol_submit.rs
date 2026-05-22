use std::collections::BTreeMap;
use std::time::Duration;

use prost::Message;
use tak_core::model::{ResolvedTask, TaskLabel};
use tak_proto::SubmitTaskResponse;

use super::{RemoteWorkspaceStage, StrictRemoteTarget};

use crate::remote_protocol_codec::{RemoteSubmitPayloadInput, build_remote_submit_payload};

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
pub(crate) struct RemoteProtocolSubmit<'a> {
    pub(crate) target: &'a StrictRemoteTarget,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) task: &'a ResolvedTask,
    pub(crate) remote_workspace: &'a RemoteWorkspaceStage,
    pub(crate) session: Option<&'a super::session_workspaces::PreparedTaskSession>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
}

pub(crate) async fn remote_protocol_submit(
    submit: RemoteProtocolSubmit<'_>,
) -> std::result::Result<(), RemoteSubmitFailure> {
    let body = build_remote_submit_payload(RemoteSubmitPayloadInput {
        target: submit.target,
        task_run_id: submit.task_run_id,
        attempt: submit.attempt,
        task: submit.task,
        remote_workspace: submit.remote_workspace,
        session: submit.session,
        execution_label: submit.execution_label,
        fused_members: submit.fused_members,
        fused_member_execution_labels: submit.fused_member_execution_labels,
    })
    .map_err(|err| RemoteSubmitFailure {
        kind: RemoteSubmitFailureKind::Other,
        message: format!("{err:#}"),
    })?
    .encode_to_vec();
    let (status, response_body) = remote_protocol_http_request(
        submit.target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        remote_submit_timeout(),
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
                submit.target.node_id, status
            ),
        });
    }
    if status != 200 {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} submit failed with HTTP {}",
                submit.target.node_id, status
            ),
        });
    }

    let parsed =
        SubmitTaskResponse::decode(response_body.as_slice()).map_err(|_| RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} returned invalid protobuf for submit",
                submit.target.node_id
            ),
        })?;
    if !parsed.accepted {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} rejected submit for task {} attempt {}",
                submit.target.node_id, submit.task.label, submit.attempt
            ),
        });
    }
    if !parsed.remote_worker {
        return Err(RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!(
                "infra error: remote node {} returned submit acknowledgement without remote worker support",
                submit.target.node_id
            ),
        });
    }

    Ok(())
}

pub(super) fn remote_submit_timeout() -> Duration {
    Duration::from_secs(10)
}
