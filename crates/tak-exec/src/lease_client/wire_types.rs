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
