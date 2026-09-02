use base64::Engine;
use tak_proto::worker_v2::{
    DispatchAttemptRequest, WorkerAttemptIdentity, WorkerAttemptPayload, WorkerWorkspace,
    WorkerWorkspaceOverlay, payload_digest,
};

use super::*;

pub(super) fn identity(command: &DispatchCommand) -> WorkerAttemptIdentity {
    WorkerAttemptIdentity {
        run_id: command.run_id.clone(),
        job_id: command.job_id.clone(),
        node_id: command.node_id.clone(),
        authored_attempt: command.authored_attempt,
        dispatch_generation: command.dispatch_generation,
        fencing_token: command.fencing_token.clone(),
    }
}

pub(super) fn dispatch(store: &RunStore, command: &DispatchCommand) -> Result<PreparedDispatch> {
    let snapshot = store.remote_execution_snapshot(command)?;
    let archive_path = snapshot.archive_path;
    let mut overlays = snapshot
        .overlays
        .into_iter()
        .map(|overlay| {
            let content_base64 = overlay
                .blob_path
                .map(std::fs::read)
                .transpose()?
                .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes));
            Ok(WorkerWorkspaceOverlay {
                entry: overlay.entry,
                content_base64,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    overlays.sort_by(|left, right| left.entry.path.cmp(&right.entry.path));
    let payload = WorkerAttemptPayload {
        workspace: WorkerWorkspace {
            descriptor: snapshot.descriptor,
            overlays,
        },
        workspace_reuse: snapshot.workspace_reuse,
        tasks: snapshot.tasks,
        environment_values: snapshot.environment_values,
        resources: snapshot.resources,
        context_manifest: snapshot.context_manifest,
    };
    let request = DispatchAttemptRequest {
        protocol_version: 2,
        identity: identity(command),
        payload_digest: payload_digest(&payload)?,
        payload,
    };
    Ok(PreparedDispatch {
        request,
        archive_path,
    })
}

pub(super) struct PreparedDispatch {
    pub(super) request: DispatchAttemptRequest,
    pub(super) archive_path: std::path::PathBuf,
}
