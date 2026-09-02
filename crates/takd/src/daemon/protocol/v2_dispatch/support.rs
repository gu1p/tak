use tak_proto::local_daemon::v2::{ErrorResponse, Response};

use crate::daemon::run_store::RunStore;

pub(super) fn daemon_status(
    manager: &crate::daemon::lease::SharedLeaseManager,
) -> anyhow::Result<tak_proto::local_daemon::v2::DaemonStatusSnapshot> {
    let mut manager = manager
        .lock()
        .map_err(|_| anyhow::anyhow!("lease manager lock poisoned"))?;
    let status = manager.status();
    Ok(tak_proto::local_daemon::v2::DaemonStatusSnapshot {
        active_leases: status.active_leases,
        pending_requests: status.pending_requests,
        limiter_count: status.usage.len(),
    })
}

pub(super) fn remote_result(
    request_id: String,
    result: Result<Response, crate::daemon::RemoteAccessError>,
) -> Result<Response, ErrorResponse> {
    match result {
        Ok(response) => Ok(response),
        Err(crate::daemon::RemoteAccessError::UnsupportedInvite) => {
            Err(ErrorResponse::remote_invite_unsupported(request_id))
        }
        Err(crate::daemon::RemoteAccessError::ProtocolMismatch) => {
            Err(ErrorResponse::protocol_version_unsupported(request_id))
        }
        Err(crate::daemon::RemoteAccessError::Failed(error)) => {
            tracing::error!("remote onboarding failed: {error:#}");
            Err(ErrorResponse::internal(request_id))
        }
    }
}

pub(super) fn attach(
    store: &RunStore,
    request_id: &str,
    run_id: String,
    after_event: u64,
) -> anyhow::Result<Response> {
    let snapshot = store
        .attachment_snapshot(&run_id, after_event)?
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    Ok(Response::RunEvents {
        protocol_version: 2,
        request_id: request_id.to_owned(),
        run_id,
        events: snapshot.events,
        next_event: snapshot.next_event,
        state: snapshot.summary.state,
        terminal: snapshot.summary.state.is_terminal() && !snapshot.has_more,
        logs_expired: snapshot.logs_expired,
        exit_code: snapshot.summary.exit_code,
    })
}

pub(super) fn classify_error(request_id: String, error: &anyhow::Error) -> ErrorResponse {
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

pub(super) fn submitter_id() -> String {
    #[cfg(unix)]
    {
        format!("uid:{}", unsafe { libc::geteuid() })
    }
    #[cfg(not(unix))]
    {
        "local-user".to_owned()
    }
}
