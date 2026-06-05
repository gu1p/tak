use std::collections::BTreeMap;
use std::time::Duration;

use prost::Message;
use tak_core::model::{ResolvedTask, TaskLabel};
use tak_proto::SubmitTaskResponse;

use super::{RemoteWorkspaceStage, StrictRemoteTarget};

use crate::remote_protocol_codec::{RemoteSubmitPayloadInput, build_remote_submit_payload};

use super::protocol_result_http::{
    RemoteHttpResponse, remote_protocol_http_request_with_extra_headers,
};
use super::remote_submit_failure::RemoteSubmitFailure;

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
    pub(crate) output_observer: Option<&'a std::sync::Arc<dyn super::TaskOutputObserver>>,
}

pub(crate) async fn remote_protocol_submit(
    submit: RemoteProtocolSubmit<'_>,
) -> std::result::Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let workspace_upload = super::workspace_upload::upload_workspace_for_submit(
        submit.target,
        submit.task_run_id,
        submit.attempt,
        submit.remote_workspace,
        Some(&submit.task.label),
        submit.output_observer,
    )
    .await?;
    let preferred_node_header = workspace_upload
        .preferred_node_id
        .as_ref()
        .map(|node_id| ("x-tak-preferred-node", node_id.clone()))
        .into_iter()
        .collect::<Vec<_>>();
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
        workspace_upload: workspace_upload.upload.as_ref(),
    })
    .map_err(|err| RemoteSubmitFailure::other(format!("{err:#}")))?
    .encode_to_vec();
    let response = remote_protocol_http_request_with_extra_headers(
        submit.target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        remote_submit_timeout(),
        &preferred_node_header,
    )
    .await
    .map_err(|err| {
        if err.is_retryable() {
            RemoteSubmitFailure::retryable_other(err.to_string())
        } else {
            RemoteSubmitFailure::other(err.to_string())
        }
    })?;
    let status = response.status;
    let response_body = &response.body;

    if status == 401 || status == 403 {
        return Err(RemoteSubmitFailure::auth(format!(
            "infra error: remote node {} auth failed during submit with HTTP {}",
            submit.target.node_id, status
        )));
    }
    if status != 200 {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} submit failed with HTTP {}",
            submit.target.node_id, status
        )));
    }

    let parsed = SubmitTaskResponse::decode(response_body.as_slice()).map_err(|_| {
        RemoteSubmitFailure::other(format!(
            "infra error: remote node {} returned invalid protobuf for submit",
            submit.target.node_id
        ))
    })?;
    if !parsed.accepted {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} rejected submit for task {} attempt {}",
            submit.target.node_id, submit.task.label, submit.attempt
        )));
    }
    if !parsed.remote_worker {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} returned submit acknowledgement without remote worker support",
            submit.target.node_id
        )));
    }

    Ok(target_after_submit(submit.target, &response))
}

pub(super) fn remote_submit_timeout() -> Duration {
    Duration::from_secs(30)
}

fn target_after_submit(
    target: &StrictRemoteTarget,
    response: &RemoteHttpResponse,
) -> StrictRemoteTarget {
    let mut selected = target.clone();
    if let Some(task_handle) = response.daemon_task_handle.clone() {
        selected.daemon_task_handle = Some(task_handle);
    }
    if let Some(node_id) = response.daemon_peer_node_id.clone() {
        selected.node_id = node_id;
    }
    if let Some(endpoint) = response.daemon_peer_endpoint.clone() {
        selected.endpoint = endpoint;
    }
    selected
}
