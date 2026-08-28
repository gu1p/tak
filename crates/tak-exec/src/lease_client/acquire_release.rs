use super::coordination_status::{
    CoordinationQueueTracker, emit_coordination_cancellation, emit_coordination_status,
    wait_for_retry_or_cancellation,
};
use super::*;
use crate::engine::cancelled_error;

pub(crate) struct TaskLease {
    lease_id: String,
    renewal: Option<tokio::task::JoinHandle<()>>,
}

impl TaskLease {
    fn new(lease: LeaseInfo, socket_path: &Path) -> Self {
        Self {
            lease_id: lease.lease_id.clone(),
            renewal: Some(spawn_lease_renewal(lease, socket_path.to_path_buf())),
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.lease_id
    }

    pub(crate) fn stop_renewal(&mut self) {
        if let Some(renewal) = self.renewal.take() {
            renewal.abort();
        }
    }
}

impl Drop for TaskLease {
    fn drop(&mut self) {
        self.stop_renewal();
    }
}

/// Repeatedly requests a lease for a task until granted or a terminal daemon error occurs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn acquire_task_lease(
    task: &ResolvedTask,
    attempt: u32,
    options: &RunOptions,
    lease_context: &LeaseContext,
) -> Result<Option<TaskLease>> {
    let Some(socket_path) = options.lease_socket.as_ref() else {
        return Ok(None);
    };

    if task.needs.is_empty() {
        return Ok(None);
    }

    let request_id = Uuid::new_v4().to_string();
    let acquire_request = AcquireLeaseRequest {
        request_id: request_id.clone(),
        client: ClientInfo {
            user: lease_context.user.clone(),
            pid: std::process::id(),
            session_id: lease_context.session_id.clone(),
        },
        task: TaskInfo {
            label: task.label.to_string(),
            attempt,
        },
        needs: convert_needs(&task.needs),
        ttl_ms: options.lease_ttl_ms.max(1_000),
    };

    let mut queue = CoordinationQueueTracker::default();
    loop {
        let response = tokio::select! {
            response = send_daemon_request(socket_path, Request::Acquire(acquire_request.clone())) => {
                response.with_context(|| format!("lease acquire request failed for {}", task.label))?
            }
            _ = options.cancellation.cancelled() => {
                emit_coordination_cancellation(task, attempt, options, &queue)?;
                return Err(cancelled_error());
            }
        };

        match response {
            Response::LeaseGranted { lease, .. } => {
                if let Some(kind) = queue.granted() {
                    emit_coordination_status(
                        task,
                        attempt,
                        options,
                        kind,
                        None,
                        "coordination lease granted",
                    )?;
                }
                return Ok(Some(TaskLease::new(lease, socket_path)));
            }
            Response::LeasePending { pending, .. } => {
                if let Some(kind) = queue.pending(pending.queue_position) {
                    emit_coordination_status(
                        task,
                        attempt,
                        options,
                        kind,
                        Some(pending.queue_position),
                        "waiting for coordination capacity",
                    )?;
                }
                let poll_ms = options.lease_poll_interval_ms.max(10);
                if let Err(error) = wait_for_retry_or_cancellation(
                    Duration::from_millis(poll_ms),
                    &options.cancellation,
                )
                .await
                {
                    emit_coordination_cancellation(task, attempt, options, &queue)?;
                    return Err(error);
                }
            }
            Response::Error { message, .. } => {
                bail!(
                    "daemon rejected lease request for {}: {message}",
                    task.label
                )
            }
            other => bail!("unexpected response while acquiring lease: {other:?}"),
        }
    }
}

fn spawn_lease_renewal(lease: LeaseInfo, socket_path: PathBuf) -> tokio::task::JoinHandle<()> {
    let lease_id = lease.lease_id;
    let ttl_ms = lease.ttl_ms.max(1_000);
    let renew_after_ms = lease.renew_after_ms.max(1).min(ttl_ms);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(renew_after_ms)).await;
            if let Err(err) = renew_task_lease(&socket_path, &lease_id, ttl_ms).await {
                tracing::warn!(
                    lease_id = %lease_id,
                    error = %format!("{err:#}"),
                    "lease renewal failed; task will continue and daemon TTL will bound stale capacity"
                );
            }
        }
    })
}

async fn renew_task_lease(socket_path: &Path, lease_id: &str, ttl_ms: u64) -> Result<()> {
    let response = send_daemon_request(
        socket_path,
        Request::Renew(RenewLeaseRequest {
            request_id: Uuid::new_v4().to_string(),
            lease_id: lease_id.to_string(),
            ttl_ms,
        }),
    )
    .await
    .with_context(|| format!("renew request failed for lease {lease_id}"))?;

    match response {
        Response::LeaseRenewed { .. } => Ok(()),
        Response::Error { message, .. } => bail!("daemon failed to renew lease: {message}"),
        other => bail!("unexpected response while renewing lease: {other:?}"),
    }
}

/// Releases a previously granted lease id using the daemon protocol.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn release_task_lease(lease_id: &str, options: &RunOptions) -> Result<()> {
    let Some(socket_path) = options.lease_socket.as_ref() else {
        return Ok(());
    };

    let response = send_daemon_request(
        socket_path,
        Request::Release(ReleaseLeaseRequest {
            request_id: Uuid::new_v4().to_string(),
            lease_id: lease_id.to_string(),
        }),
    )
    .await
    .with_context(|| format!("release request failed for lease {lease_id}"))?;

    match response {
        Response::LeaseReleased { .. } => Ok(()),
        Response::Error { message, .. } => bail!("daemon failed to release lease: {message}"),
        other => bail!("unexpected response while releasing lease: {other:?}"),
    }
}
