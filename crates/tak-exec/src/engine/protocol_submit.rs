use std::collections::BTreeMap;
use std::time::Duration;

use prost::Message;
use tak_core::model::{ResolvedTask, TaskLabel};
use tak_proto::{SubmitTaskResponse, WorkspaceUploadRef};

use super::{RemoteWorkspaceStage, StrictRemoteTarget};

use crate::remote_protocol_codec::{RemoteSubmitPayloadInput, build_remote_submit_payload};

use super::protocol_result_http::{
    RemoteHttpResponse, remote_protocol_http_request_with_extra_headers,
};
use super::remote_submit_failure::RemoteSubmitFailure;
use super::workspace_upload_cache::{CachedUpload, SharedWorkspaceUploadCache, UploadClaim};

/// Submits one remote attempt, reusing a per-job cached workspace upload when possible.
///
/// `remote_workspace` carries the staged archive when one already exists (the miss path /
/// auth fallback). It may be `None` on the cache-hit path where staging was skipped: if this
/// call nonetheless needs to upload (the cached blob vanished, or it became the single-flight
/// leader), it returns [`RemoteSubmitFailure::missing_upload`] so the caller stages and
/// retries. See the per-job upload cache in `workspace_upload_cache`.
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
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(crate) session: Option<&'a super::session_workspaces::PreparedTaskSession>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
    pub(crate) output_observer: Option<&'a std::sync::Arc<dyn super::TaskOutputObserver>>,
    pub(crate) upload_cache: &'a SharedWorkspaceUploadCache,
    pub(crate) workspace_content_hash: &'a str,
}

/// Submit metadata shared by every POST, independent of how the workspace is conveyed.
struct SubmitPost<'a> {
    target: &'a StrictRemoteTarget,
    task_run_id: &'a str,
    attempt: u32,
    task: &'a ResolvedTask,
    session: Option<&'a super::session_workspaces::PreparedTaskSession>,
    fused_members: Option<&'a [ResolvedTask]>,
    execution_label: Option<&'a str>,
    fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
}

impl<'a> RemoteProtocolSubmit<'a> {
    fn post(&self) -> SubmitPost<'a> {
        SubmitPost {
            target: self.target,
            task_run_id: self.task_run_id,
            attempt: self.attempt,
            task: self.task,
            session: self.session,
            fused_members: self.fused_members,
            execution_label: self.execution_label,
            fused_member_execution_labels: self.fused_member_execution_labels,
        }
    }
}

pub(crate) async fn remote_protocol_submit(
    submit: RemoteProtocolSubmit<'_>,
) -> std::result::Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let key = (
        submit.target.node_id.clone(),
        submit.workspace_content_hash.to_string(),
    );
    let post = submit.post();
    // At most two claim rounds: a reused reference that the node reports missing triggers one
    // fresh upload (when this call has a stage to upload from).
    for round in 0..2 {
        match submit.upload_cache.claim(key.clone()).await {
            UploadClaim::Reuse(cached) => {
                tracing::debug!(
                    node_id = %submit.target.node_id,
                    upload_id = %cached.upload.upload_id,
                    archive_bytes = cached.archive_byte_len,
                    "reusing cached workspace upload for task {} attempt {}",
                    submit.task.label,
                    submit.attempt,
                );
                match post_submit(
                    &post,
                    Some(&cached.upload),
                    cached.preferred_node_id.as_deref(),
                    None,
                )
                .await
                {
                    Ok(target) => return Ok(target),
                    Err(err) if err.is_missing_upload() => {
                        submit.upload_cache.invalidate(&key);
                        if submit.remote_workspace.is_some() && round == 0 {
                            continue; // re-claim; this call will upload a fresh blob
                        }
                        return Err(err); // no stage here — caller stages and retries
                    }
                    Err(err) => return Err(err),
                }
            }
            UploadClaim::Lead(guard) => {
                let Some(stage) = submit.remote_workspace else {
                    // We became the leader but have no staged workspace to upload. Release the
                    // slot and ask the caller to stage and retry.
                    drop(guard);
                    return Err(RemoteSubmitFailure::missing_upload(format!(
                        "remote node {} has no cached workspace upload; staging required",
                        submit.target.node_id
                    )));
                };
                let outcome = match super::workspace_upload::upload_workspace_for_submit(
                    submit.target,
                    submit.task_run_id,
                    submit.attempt,
                    stage,
                    Some(&submit.task.label),
                    submit.output_observer,
                )
                .await
                {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        drop(guard); // clears the slot so waiters re-claim
                        return Err(err);
                    }
                };
                match outcome.upload {
                    Some(upload) => {
                        guard.publish(CachedUpload {
                            upload: upload.clone(),
                            preferred_node_id: outcome.preferred_node_id.clone(),
                            archive_byte_len: stage.archive_byte_len,
                        });
                        match post_submit(
                            &post,
                            Some(&upload),
                            outcome.preferred_node_id.as_deref(),
                            None,
                        )
                        .await
                        {
                            Ok(target) => return Ok(target),
                            Err(err) if err.is_missing_upload() && round == 0 => {
                                // The blob we just uploaded was reaped (or the preferred worker
                                // dropped) before this submit landed. Drop the now-stale cache
                                // entry and re-upload once with the staged archive we still hold.
                                submit.upload_cache.invalidate(&key);
                                continue;
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    None => {
                        // Inline transport result: not a shared blob, so it cannot be cached.
                        drop(guard);
                        return post_submit(&post, None, None, Some(stage)).await;
                    }
                }
            }
        }
    }
    Err(RemoteSubmitFailure::other(format!(
        "infra error: remote node {} workspace upload reuse did not converge",
        submit.target.node_id
    )))
}

async fn post_submit(
    post: &SubmitPost<'_>,
    workspace_upload: Option<&WorkspaceUploadRef>,
    preferred_node_id: Option<&str>,
    inline_stage: Option<&RemoteWorkspaceStage>,
) -> std::result::Result<StrictRemoteTarget, RemoteSubmitFailure> {
    let preferred_node_header = preferred_node_id
        .map(|node_id| ("x-tak-preferred-node", node_id.to_string()))
        .into_iter()
        .collect::<Vec<_>>();
    let body = build_remote_submit_payload(RemoteSubmitPayloadInput {
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
    .map_err(|err| RemoteSubmitFailure::other(format!("{err:#}")))?
    .encode_to_vec();
    let response = remote_protocol_http_request_with_extra_headers(
        post.target,
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
            post.target.node_id, status
        )));
    }
    if status == 409 {
        // The referenced workspace upload was reaped on the node; re-upload and resubmit.
        return Err(RemoteSubmitFailure::missing_upload(format!(
            "remote node {} reports referenced workspace upload missing (HTTP 409)",
            post.target.node_id
        )));
    }
    if status != 200 {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} submit failed with HTTP {}",
            post.target.node_id, status
        )));
    }

    let parsed = SubmitTaskResponse::decode(response_body.as_slice()).map_err(|_| {
        RemoteSubmitFailure::other(format!(
            "infra error: remote node {} returned invalid protobuf for submit",
            post.target.node_id
        ))
    })?;
    if !parsed.accepted {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} rejected submit for task {} attempt {}",
            post.target.node_id, post.task.label, post.attempt
        )));
    }
    if !parsed.remote_worker {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} returned submit acknowledgement without remote worker support",
            post.target.node_id
        )));
    }

    Ok(target_after_submit(post.target, &response))
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
