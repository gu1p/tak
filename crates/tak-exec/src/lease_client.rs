use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tak_core::model::{NeedDef, ResolvedTask, Scope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

use crate::{LeaseContext, RunOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientInfo {
    user: String,
    pid: u32,
    session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskInfo {
    label: String,
    attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NeedRequest {
    pub(crate) name: String,
    pub(crate) scope: Scope,
    pub(crate) scope_key: Option<String>,
    pub(crate) slots: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AcquireLeaseRequest {
    request_id: String,
    client: ClientInfo,
    task: TaskInfo,
    needs: Vec<NeedRequest>,
    ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseLeaseRequest {
    request_id: String,
    lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaseInfo {
    lease_id: String,
    ttl_ms: u64,
    renew_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingInfo {
    queue_position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Request {
    AcquireLease(AcquireLeaseRequest),
    ReleaseLease(ReleaseLeaseRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Response {
    LeaseGranted {
        request_id: String,
        lease: LeaseInfo,
    },
    LeasePending {
        request_id: String,
        pending: PendingInfo,
    },
    LeaseReleased {
        request_id: String,
    },
    Error {
        request_id: String,
        message: String,
    },
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
) -> Result<Option<String>> {
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

    loop {
        let response =
            send_daemon_request(socket_path, Request::AcquireLease(acquire_request.clone()))
                .await
                .with_context(|| format!("lease acquire request failed for {}", task.label))?;

        match response {
            Response::LeaseGranted { lease, .. } => return Ok(Some(lease.lease_id)),
            Response::LeasePending { .. } => {
                let poll_ms = options.lease_poll_interval_ms.max(10);
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
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
        Request::ReleaseLease(ReleaseLeaseRequest {
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

/// Converts core model need definitions into daemon wire-format needs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn convert_needs(needs: &[NeedDef]) -> Vec<NeedRequest> {
    needs
        .iter()
        .map(|need| NeedRequest {
            name: need.limiter.name.clone(),
            scope: need.limiter.scope.clone(),
            scope_key: need.limiter.scope_key.clone(),
            slots: need.slots,
        })
        .collect()
}

/// Sends one NDJSON request to the daemon and returns the decoded response frame.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn send_daemon_request(socket_path: &Path, request: Request) -> Result<Response> {
    let stream = UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();

    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before response");
    }

    serde_json::from_str(line.trim_end()).context("failed to decode daemon response")
}
