use std::time::Duration;

use prost::Message;
use tak_proto::{SubmitTaskResponse, WorkspaceUploadRef};

use super::{RemoteSubmitFailure, RemoteWorkspaceStage, StrictRemoteTarget, SubmitPost};
use crate::engine::protocol_result_http::{
    RemoteHttpResponse, remote_protocol_http_request_with_extra_headers,
};
use crate::remote_protocol_codec::{RemoteSubmitPayloadInput, build_remote_submit_payload};

#[cfg(test)]
mod response_tests;

pub(super) async fn post_submit(
    post: &SubmitPost<'_>,
    workspace_upload: Option<&WorkspaceUploadRef>,
    preferred_node_id: Option<&str>,
    inline_stage: Option<&RemoteWorkspaceStage>,
) -> Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let headers = preferred_node_id
        .map(|node_id| ("x-tak-preferred-node", node_id.to_string()))
        .into_iter()
        .collect::<Vec<_>>();
    let body = submit_body(post, workspace_upload, inline_stage)?;
    let response = remote_protocol_http_request_with_extra_headers(
        post.target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        remote_submit_timeout(),
        &headers,
    )
    .await
    .map_err(transport_failure)?;
    validate_response(post, response)
}

fn submit_body(
    post: &SubmitPost<'_>,
    workspace_upload: Option<&WorkspaceUploadRef>,
    inline_stage: Option<&RemoteWorkspaceStage>,
) -> Result<Vec<u8>, RemoteSubmitFailure> {
    build_remote_submit_payload(RemoteSubmitPayloadInput {
        target: post.target,
        task_run_id: post.task_run_id,
        attempt: post.attempt,
        task: post.task,
        remote_workspace: inline_stage,
        session: post.session,
        execution_label: post.execution_label,
        fused_members: post.fused_members,
        fused_member_execution_labels: post.fused_member_execution_labels,
        workspace_upload,
    })
    .map_err(|error| RemoteSubmitFailure::other(format!("{error:#}")))
    .map(|payload| payload.encode_to_vec())
}

fn transport_failure(error: crate::engine::RemoteHttpExchangeError) -> RemoteSubmitFailure {
    let failed_node_id = error.failed_node_id().map(str::to_string);
    let message = error.to_string();
    let failure = if error.is_retryable() {
        RemoteSubmitFailure::retryable_other(message)
    } else {
        RemoteSubmitFailure::other(message)
    };
    if let Some(node_id) = failed_node_id {
        return failure.with_failed_node_id(node_id);
    }
    failure
}

fn validate_response(
    post: &SubmitPost<'_>,
    response: RemoteHttpResponse,
) -> Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let failure = |failure| identify_failure(failure, &response);
    let node_id = response_node_id(post.target, &response);
    match response.status {
        401 | 403 => Err(failure(RemoteSubmitFailure::auth(format!(
            "infra error: remote node {} auth failed during submit with HTTP {}",
            node_id, response.status
        )))),
        409 => Err(failure(RemoteSubmitFailure::missing_upload(format!(
            "remote node {} reports referenced workspace upload missing (HTTP 409)",
            node_id
        )))),
        200 => validate_acknowledgement(post, response),
        status => Err(failure(submit_status_failure(status, &node_id))),
    }
}

fn submit_status_failure(status: u16, node_id: &str) -> RemoteSubmitFailure {
    let message = format!("infra error: remote node {node_id} submit failed with HTTP {status}");
    if matches!(status, 408 | 429 | 500..=599) {
        return RemoteSubmitFailure::retryable_other(message);
    }
    RemoteSubmitFailure::other(message)
}

fn validate_acknowledgement(
    post: &SubmitPost<'_>,
    response: RemoteHttpResponse,
) -> Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let node_id = response_node_id(post.target, &response);
    let parsed = SubmitTaskResponse::decode(response.body.as_slice()).map_err(|_| {
        identify_failure(
            RemoteSubmitFailure::other(format!(
                "infra error: remote node {node_id} returned invalid protobuf for submit"
            )),
            &response,
        )
    })?;
    if !parsed.accepted || !parsed.remote_worker {
        let detail = if parsed.accepted {
            "returned submit acknowledgement without remote worker support".to_string()
        } else {
            format!(
                "rejected submit for task {} attempt {}",
                post.task.label, post.attempt
            )
        };
        return Err(identify_failure(
            RemoteSubmitFailure::other(format!("infra error: remote node {node_id} {detail}")),
            &response,
        ));
    }
    Ok(target_after_submit(post.target, &response))
}

fn response_node_id(target: &StrictRemoteTarget, response: &RemoteHttpResponse) -> String {
    response
        .daemon_peer_node_id
        .clone()
        .unwrap_or_else(|| target.node_id.clone())
}

fn identify_failure(
    failure: RemoteSubmitFailure,
    response: &RemoteHttpResponse,
) -> RemoteSubmitFailure {
    if let Some(node_id) = response.daemon_peer_node_id.as_deref() {
        return failure.with_failed_node_id(node_id);
    }
    failure
}

pub(crate) fn remote_submit_timeout() -> Duration {
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
