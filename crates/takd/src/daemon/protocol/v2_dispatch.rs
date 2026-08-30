use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::{ErrorResponse, Operation, Request, Response};

use crate::daemon::run_store::RunStore;

pub(super) fn dispatch(request: Request, store: &RunStore) -> Result<Response, ErrorResponse> {
    let request_id = request.request_id;
    let result = match request.operation {
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
            .summary(&run_id)
            .and_then(|run| run.ok_or_else(|| anyhow::anyhow!("run not found")))
            .map(|_| Response::OutputManifest {
                protocol_version: 2,
                request_id: request_id.clone(),
                run_id,
                expired: false,
                artifacts: Vec::new(),
            }),
        Operation::GetOutputChunk { .. } => Err(anyhow::anyhow!("artifact not found")),
    };
    result.map_err(|error| classify_error(request_id, &error))
}

fn attach(
    store: &RunStore,
    request_id: &str,
    run_id: String,
    after_event: u64,
) -> anyhow::Result<Response> {
    let (summary, events, has_more) = store
        .attachment_snapshot(&run_id, after_event)?
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    let next_event = events.last().map_or(after_event, |event| event.seq);
    Ok(Response::RunEvents {
        protocol_version: 2,
        request_id: request_id.to_owned(),
        run_id,
        events,
        next_event,
        state: summary.state,
        terminal: summary.state.is_terminal() && !has_more,
    })
}

fn classify_error(request_id: String, error: &anyhow::Error) -> ErrorResponse {
    let message = error.to_string();
    if message.contains("idempotency conflict") {
        ErrorResponse::idempotency_conflict(request_id)
    } else if message.contains("not found") {
        ErrorResponse::run_not_found(request_id)
    } else if message.contains("workspace") || message.contains("archive") {
        ErrorResponse::workspace_invalid(request_id)
    } else if message.contains("state") || message.contains("incomplete") {
        ErrorResponse::run_state_invalid(request_id)
    } else {
        tracing::error!("protocol v2 run operation failed: {error:#}");
        ErrorResponse::internal(request_id)
    }
}

fn submitter_id() -> String {
    #[cfg(unix)]
    {
        format!("uid:{}", unsafe { libc::geteuid() })
    }
    #[cfg(not(unix))]
    {
        "local-user".to_owned()
    }
}
