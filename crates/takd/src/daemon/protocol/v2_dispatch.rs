use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::{ErrorResponse, Operation, Request, Response};

use crate::daemon::run_store::RunStore;

mod support;

use support::{attach, classify_error, daemon_status, remote_result, submitter_id};

pub(super) async fn dispatch(
    request: Request,
    manager: &crate::daemon::lease::SharedLeaseManager,
    store: &RunStore,
    peers: &crate::daemon::peer_manager::PeerManager,
    remote_access: &crate::daemon::RemoteAccess,
) -> Result<Response, ErrorResponse> {
    let request_id = request.request_id;
    let result =
        match request.operation {
            Operation::GetDaemonStatus {} => {
                daemon_status(manager).map(|status| Response::DaemonStatus {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    status,
                })
            }
            Operation::PreviewRemote { invite } => {
                return remote_result(
                    request_id.clone(),
                    remote_access
                        .preview(&invite)
                        .await
                        .map(|remote| Response::RemotePreview {
                            protocol_version: 2,
                            request_id,
                            remote,
                        }),
                );
            }
            Operation::AddRemote { invite } => {
                return remote_result(
                    request_id.clone(),
                    remote_access
                        .add(&invite)
                        .await
                        .map(|remote| Response::RemoteAdded {
                            protocol_version: 2,
                            request_id,
                            remote,
                        }),
                );
            }
            Operation::ListRemotes {} => remote_access.list().map(|remotes| Response::RemoteList {
                protocol_version: 2,
                request_id: request_id.clone(),
                remotes,
            }),
            Operation::RemoveRemote { node_id } => {
                remote_access
                    .remove(&node_id)
                    .await
                    .map(|removed| Response::RemoteRemoved {
                        protocol_version: 2,
                        request_id: request_id.clone(),
                        node_id,
                        removed,
                    })
            }
            Operation::GetRemoteStatus { node_ids } => {
                remote_access
                    .statuses(&node_ids)
                    .await
                    .map(|remotes| Response::RemoteStatus {
                        protocol_version: 2,
                        request_id: request_id.clone(),
                        remotes,
                    })
            }
            Operation::ReadRemote { node_id, path } => remote_access
                .read(&node_id, &path)
                .await
                .map(|(http_status, body)| Response::RemoteRead {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    node_id,
                    http_status,
                    body_base64: STANDARD.encode(body),
                }),
            Operation::ResolveRemoteCandidates { requirements } => Ok(Response::RemoteCandidates {
                protocol_version: 2,
                request_id: request_id.clone(),
                candidates: peers.remote_candidates(&requirements),
            }),
            Operation::SubmitRun {
                idempotency_key,
                run,
                environment_values,
            } => RunSubmission::new(idempotency_key, *run, environment_values)
                .map_err(anyhow::Error::from)
                .and_then(|submission| store.submit(&submission, &submitter_id()))
                .map(|submitted| Response::RunSubmitted {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    run_id: submitted.run_id,
                    workspace: submitted.workspace,
                }),
            Operation::UploadWorkspace {
                run_id,
                workspace_fingerprint,
                archive_size,
                offset,
                chunk,
            } => store
                .upload_workspace(
                    &run_id,
                    &workspace_fingerprint,
                    archive_size,
                    offset,
                    &chunk,
                )
                .map(|progress| Response::WorkspaceUploadProgress {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    run_id,
                    workspace_fingerprint,
                    chunk_accepted: progress.chunk_accepted,
                    next_offset: progress.next_offset,
                    complete: progress.complete,
                }),
            Operation::CommitRun { run_id } => {
                store.commit(&run_id).map(|summary| Response::RunCommitted {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    run_id,
                    state: summary.state,
                })
            }
            Operation::ListRuns {} => store.list_runs().map(|runs| Response::RunList {
                protocol_version: 2,
                request_id: request_id.clone(),
                runs,
            }),
            Operation::GetRun { run_id } => store
                .get_run(&run_id)
                .and_then(|run| run.ok_or_else(|| anyhow::anyhow!("run not found")))
                .map(|run| Response::RunDetails {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    run,
                }),
            Operation::AttachRun {
                run_id,
                after_event,
            } => attach(store, &request_id, run_id, after_event),
            Operation::CancelRun { run_id } => {
                store
                    .cancel(&run_id)
                    .map(|state| Response::CancellationAccepted {
                        protocol_version: 2,
                        request_id: request_id.clone(),
                        run_id,
                        state,
                    })
            }
            Operation::GetOutputManifest { run_id } => store
                .output_manifest_status(&run_id)
                .and_then(|manifest| manifest.ok_or_else(|| anyhow::anyhow!("run not found")))
                .map(|manifest| Response::OutputManifest {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    run_id,
                    expired: manifest.expired,
                    artifacts: manifest.artifacts,
                }),
            Operation::GetOutputChunk {
                artifact_id,
                offset,
                max_bytes,
            } => store
                .output_chunk(&artifact_id, offset, max_bytes)
                .and_then(|chunk| chunk.ok_or_else(|| anyhow::anyhow!("artifact not found")))
                .map(|chunk| Response::OutputChunk {
                    protocol_version: 2,
                    request_id: request_id.clone(),
                    artifact_id,
                    offset,
                    chunk_base64: STANDARD.encode(chunk.bytes),
                    complete: chunk.complete,
                }),
        };
    result.map_err(|error| classify_error(request_id, &error))
}
