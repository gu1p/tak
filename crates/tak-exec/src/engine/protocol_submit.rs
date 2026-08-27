use std::collections::BTreeMap;

use tak_core::model::{ResolvedTask, TaskLabel};

use super::{RemoteWorkspaceStage, StrictRemoteTarget};

use super::remote_submit_failure::RemoteSubmitFailure;
use super::workspace_upload_cache::{CachedUpload, SharedWorkspaceUploadCache, UploadClaim};

#[path = "protocol_submit/response.rs"]
pub(super) mod response;
use response::post_submit;

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
