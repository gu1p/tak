use anyhow::{Result, anyhow};
use serde::Serialize;
use tak_core::model::{RemoteRuntimeSpec, StepDef};
use tak_proto::{NodeInfo, NodeStatusResponse};

use super::query_helpers::remote_execution_root_base;
use super::status_state::{ActiveJobMetadata, SharedNodeStatusState, new_shared_node_status_state};

#[derive(Debug, Clone)]
pub(super) struct RemoteWorkerSubmitPayload {
    pub(super) workspace_zip: Vec<u8>,
    pub(super) task_label: String,
    pub(super) attempt: u32,
    pub(super) steps: Vec<StepDef>,
    pub(super) timeout_s: Option<u64>,
    pub(super) runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RemoteWorkerOutputRecord {
    pub(super) path: String,
    pub(super) digest: String,
    pub(super) size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceFileFingerprint {
    pub(super) digest: String,
    pub(super) size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitEventRecord {
    pub seq: u64,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteV1Response {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct RemoteNodeContext {
    pub node: NodeInfo,
    pub bearer_token: String,
    status_state: SharedNodeStatusState,
}

impl RemoteNodeContext {
    pub fn new(node: NodeInfo, bearer_token: String) -> Self {
        Self {
            node,
            bearer_token,
            status_state: new_shared_node_status_state(remote_execution_root_base()),
        }
    }

    pub(crate) fn register_active_job(
        &self,
        idempotency_key: String,
        job: ActiveJobMetadata,
    ) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.register_job(idempotency_key, job);
        Ok(())
    }

    pub(crate) fn finish_active_job(&self, idempotency_key: &str) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.finish_job(idempotency_key);
        Ok(())
    }

    pub(crate) fn node_status(&self) -> Result<NodeStatusResponse> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.snapshot(&self.node)
    }

    pub(crate) fn shared_status_state(&self) -> SharedNodeStatusState {
        self.status_state.clone()
    }
}
