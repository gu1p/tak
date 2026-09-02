use base64::Engine as _;
use prost::Message as _;

use super::identifier::is_valid_identifier;
use super::{Response, RunEventKind, RunLifecycleState, RunSummary};

pub(super) fn valid_success(response: &Response) -> bool {
    match response {
        Response::Error { .. } => false,
        Response::DaemonStatus { .. } => true,
        Response::RemotePreview { remote, .. } | Response::RemoteAdded { remote, .. } => {
            valid_remote(remote)
        }
        Response::RemoteList { remotes, .. } => valid_remote_list(remotes),
        Response::RemoteRemoved { node_id, .. } => is_valid_identifier(node_id),
        Response::RemoteStatus { remotes, .. } => {
            let mut nodes = std::collections::BTreeSet::new();
            remotes.iter().all(|status| {
                valid_remote(&status.remote)
                    && nodes.insert(status.remote.node_id.as_str())
                    && valid_remote_status(status)
            })
        }
        Response::RemoteRead {
            node_id,
            http_status,
            body_base64,
            ..
        } => {
            is_valid_identifier(node_id)
                && (100..=599).contains(http_status)
                && base64::engine::general_purpose::STANDARD
                    .decode(body_base64)
                    .is_ok()
        }
        Response::RemoteCandidates { candidates, .. } => {
            let mut nodes = std::collections::BTreeSet::new();
            candidates.iter().all(|candidate| {
                is_valid_identifier(&candidate.node_id)
                    && nodes.insert(candidate.node_id.as_str())
                    && candidate.kind == tak_core::v2::PlacementKind::Remote
                    && matches!(candidate.transport.as_deref(), Some("direct" | "tor"))
                    && !candidate.reason.trim().is_empty()
            })
        }
        Response::RunSubmitted { run_id, .. }
        | Response::WorkspaceUploadProgress { run_id, .. }
        | Response::RunCommitted { run_id, .. }
        | Response::CancellationAccepted { run_id, .. } => is_valid_identifier(run_id),
        Response::OutputManifest {
            run_id,
            expired,
            artifacts,
            ..
        } => is_valid_identifier(run_id) && (!expired || artifacts.is_empty()),
        Response::RunEvents {
            run_id,
            events,
            next_event,
            state,
            terminal,
            logs_expired,
            exit_code,
            ..
        } => {
            is_valid_identifier(run_id)
                && valid_sequences(events, *next_event, *logs_expired)
                && events.iter().all(valid_event)
                && (!logs_expired || events.iter().all(payload_is_redacted))
                && (!terminal || state.is_terminal())
                && terminal_exit_code_is_valid(*state, *terminal, *exit_code)
        }
        Response::RunList { runs, .. } => runs.iter().all(valid_summary),
        Response::RunDetails { run, .. } => valid_summary(&run.summary),
        Response::OutputChunk { artifact_id, .. } => is_valid_identifier(artifact_id),
    }
}

fn valid_remote_list(remotes: &[super::RemoteInventoryEntry]) -> bool {
    let mut nodes = std::collections::BTreeSet::new();
    remotes
        .iter()
        .all(|remote| valid_remote(remote) && nodes.insert(remote.node_id.as_str()))
}

fn valid_remote(remote: &super::RemoteInventoryEntry) -> bool {
    is_valid_identifier(&remote.node_id)
        && !remote.base_url.trim().is_empty()
        && matches!(remote.transport.as_str(), "direct" | "tor")
}

fn valid_remote_status(status: &super::RemoteStatusEntry) -> bool {
    match (
        status.snapshot.as_ref(),
        status.detail_base64.as_deref(),
        status.peer.as_ref(),
        status.error.as_deref(),
    ) {
        (Some(snapshot), detail, None, None) => {
            snapshot.node_id == status.remote.node_id
                && crate::worker_v2::encode_snapshot(snapshot).is_ok()
                && detail
                    .is_none_or(|payload| valid_status_payload(payload, &status.remote.node_id))
        }
        (None, None, Some(peer), error) => {
            peer.node_id == status.remote.node_id
                && peer.transport == status.remote.transport
                && error.is_none_or(|message| !message.trim().is_empty())
        }
        (None, None, None, Some(message)) => !message.trim().is_empty(),
        _ => false,
    }
}

fn valid_status_payload(payload: &str, node_id: &str) -> bool {
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(payload) else {
        return false;
    };
    crate::NodeStatusResponse::decode(bytes.as_slice())
        .ok()
        .and_then(|status| status.node)
        .is_some_and(|node| node.node_id == node_id)
}

fn valid_sequences(events: &[super::RunEvent], next_event: u64, logs_expired: bool) -> bool {
    events.windows(2).all(|pair| pair[0].seq < pair[1].seq)
        && events.last().is_none_or(|event| {
            if logs_expired {
                event.seq <= next_event
            } else {
                event.seq == next_event
            }
        })
}

fn valid_event(event: &super::RunEvent) -> bool {
    let is_output = matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr);
    is_output == event.chunk_base64.is_some()
        && event_exit_code_is_valid(event.kind, event.exit_code)
        && event.chunk_base64.as_ref().is_none_or(|chunk| {
            base64::engine::general_purpose::STANDARD
                .decode(chunk)
                .is_ok()
        })
}

fn payload_is_redacted(event: &super::RunEvent) -> bool {
    !matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr)
        && event.chunk_base64.is_none()
}

fn valid_summary(summary: &RunSummary) -> bool {
    is_valid_identifier(&summary.run_id)
        && terminal_exit_code_is_valid(
            summary.state,
            summary.state.is_terminal(),
            summary.exit_code,
        )
}

fn event_exit_code_is_valid(kind: RunEventKind, exit_code: Option<i32>) -> bool {
    match kind {
        RunEventKind::Succeeded => exit_code.is_none_or(|code| code == 0),
        RunEventKind::Failed => exit_code.is_none_or(valid_failure_code),
        _ => exit_code.is_none(),
    }
}

fn terminal_exit_code_is_valid(
    state: RunLifecycleState,
    terminal: bool,
    exit_code: Option<i32>,
) -> bool {
    if !terminal {
        return exit_code.is_none();
    }
    match state {
        RunLifecycleState::Succeeded => exit_code.is_none_or(|code| code == 0),
        RunLifecycleState::Failed => exit_code.is_none_or(valid_failure_code),
        RunLifecycleState::Cancelled => exit_code.is_none(),
        _ => false,
    }
}

fn valid_failure_code(code: i32) -> bool {
    (1..=u8::MAX.into()).contains(&code)
}
