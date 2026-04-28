use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use serde::Serialize;
use tak_core::model::{OutputSelectorSpec, RemoteRuntimeSpec, StepDef};
use tak_proto::{NodeInfo, NodeStatusResponse};

use super::execution_root::remote_execution_root_base;
use super::runtime::RemoteRuntimeConfig;
use super::runtime_state::RemoteRuntimeState;
use super::status_state::{ActiveJobMetadata, SharedNodeStatusState, new_shared_node_status_state};

#[derive(Debug, Clone)]
pub(super) struct RemoteWorkerSubmitPayload {
    pub(super) workspace_zip: Vec<u8>,
    pub(super) task_label: String,
    pub(super) attempt: u32,
    pub(super) steps: Vec<StepDef>,
    pub(super) timeout_s: Option<u64>,
    pub(super) runtime: Option<RemoteRuntimeSpec>,
    pub(super) outputs: Vec<OutputSelectorSpec>,
    pub(super) session: Option<RemoteWorkerSession>,
}

#[derive(Debug, Clone)]
pub(super) struct RemoteWorkerSession {
    pub(super) key: String,
    pub(super) reuse: RemoteWorkerSessionReuse,
}

#[derive(Debug, Clone)]
pub(super) enum RemoteWorkerSessionReuse {
    ShareWorkspace,
    SharePaths { paths: Vec<OutputSelectorSpec> },
}

#[derive(Debug, Clone)]
pub struct RemoteImageCacheRuntimeConfig {
    pub db_path: PathBuf,
    pub budget_bytes: u64,
    pub mutable_tag_ttl_secs: u64,
    pub sweep_interval_secs: u64,
    pub low_disk_min_free_percent: f64,
    pub low_disk_min_free_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RemoteWorkerOutputRecord {
    pub(super) path: String,
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
    node: Arc<Mutex<NodeInfo>>,
    pub bearer_token: String,
    status_state: SharedNodeStatusState,
    runtime_state: Arc<RemoteRuntimeState>,
    image_cache: Option<RemoteImageCacheRuntimeConfig>,
}

impl RemoteNodeContext {
    pub fn new(node: NodeInfo, bearer_token: String, runtime_config: RemoteRuntimeConfig) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
            bearer_token,
            status_state: new_shared_node_status_state(),
            runtime_state: Arc::new(RemoteRuntimeState::new(runtime_config)),
            image_cache: None,
        }
    }

    pub fn with_image_cache_config(mut self, config: RemoteImageCacheRuntimeConfig) -> Self {
        self.image_cache = Some(config);
        self
    }

    pub fn node_info(&self) -> Result<NodeInfo> {
        self.node
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| anyhow!("remote node lock poisoned"))
    }

    pub fn mark_transport_ready(&self) -> Result<()> {
        self.set_transport_state("ready", None)
    }

    pub fn set_transport_state(
        &self,
        transport_state: &str,
        transport_detail: Option<&str>,
    ) -> Result<()> {
        let mut guard = self
            .node
            .lock()
            .map_err(|_| anyhow!("remote node lock poisoned"))?;
        if guard.transport != "tor" {
            guard.healthy = true;
            guard.transport_state = "ready".to_string();
            guard.transport_detail.clear();
            return Ok(());
        }
        guard.healthy = transport_state == "ready";
        guard.transport_state = transport_state.to_string();
        guard.transport_detail = transport_detail.unwrap_or_default().to_string();
        Ok(())
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
        let node = self.node_info()?;
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.snapshot(
            &node,
            &remote_execution_root_base(self),
            self.image_cache.as_ref(),
        )
    }

    pub(crate) fn shared_status_state(&self) -> SharedNodeStatusState {
        self.status_state.clone()
    }

    pub fn runtime_config(&self) -> RemoteRuntimeConfig {
        self.runtime_state.config.clone()
    }

    pub(crate) fn image_cache_config(&self) -> Option<RemoteImageCacheRuntimeConfig> {
        self.image_cache.clone()
    }

    pub(crate) fn runtime_state(&self) -> &Arc<RemoteRuntimeState> {
        &self.runtime_state
    }
}
