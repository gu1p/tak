use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use tak_proto::NodeInfo;

use super::active_executions::SharedActiveExecutions;
use super::resource_admission::{
    ResourceAdmissionDecision, ResourceRequest, SharedResourceAdmission,
};
use super::tak_container_usage::SharedTakContainerUsage;

#[path = "types_tests.rs"]
mod tests;
use super::runtime::RemoteRuntimeConfig;
use super::runtime_state::RemoteRuntimeState;
use super::status_state::{SharedNodeStatusState, new_shared_node_status_state};

mod context_status;
mod records;
mod worker_payload;

pub use records::{RemoteV1Response, SubmitAttemptSummaryRecord, SubmitEventRecord};
pub use worker_payload::RemoteImageCacheRuntimeConfig;
pub(super) use worker_payload::{
    RemoteWorkerFusedMember, RemoteWorkerOutputRecord, RemoteWorkerSession,
    RemoteWorkerSessionReuse, RemoteWorkerSubmitPayload,
};

#[derive(Clone)]
pub struct RemoteNodeContext {
    node: Arc<Mutex<NodeInfo>>,
    pub bearer_token: String,
    status_state: SharedNodeStatusState,
    active_executions: SharedActiveExecutions,
    resource_admission: SharedResourceAdmission,
    tak_container_usage: SharedTakContainerUsage,
    runtime_state: Arc<RemoteRuntimeState>,
    image_cache: Option<RemoteImageCacheRuntimeConfig>,
    state_root: Option<PathBuf>,
}

impl RemoteNodeContext {
    pub fn new(node: NodeInfo, bearer_token: String, runtime_config: RemoteRuntimeConfig) -> Self {
        let tak_container_usage = SharedTakContainerUsage::default();
        Self {
            node: Arc::new(Mutex::new(node)),
            bearer_token,
            status_state: new_shared_node_status_state(tak_container_usage.clone()),
            active_executions: SharedActiveExecutions::default(),
            resource_admission: SharedResourceAdmission::new_detected(tak_container_usage.clone()),
            tak_container_usage,
            runtime_state: Arc::new(RemoteRuntimeState::new(runtime_config)),
            image_cache: None,
            state_root: None,
        }
    }

    pub fn with_image_cache_config(mut self, config: RemoteImageCacheRuntimeConfig) -> Self {
        self.image_cache = Some(config);
        self
    }

    pub fn with_state_root(mut self, state_root: &std::path::Path) -> Self {
        self.state_root = Some(state_root.to_path_buf());
        self
    }

    pub(crate) fn state_root(&self) -> Option<PathBuf> {
        self.state_root.clone()
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

    pub(crate) fn register_active_execution(
        &self,
        idempotency_key: String,
        task_run_id: &str,
        attempt: u32,
    ) -> Result<tak_runner::RunCancellation> {
        self.active_executions
            .register(idempotency_key, task_run_id, attempt)
    }

    pub(crate) fn unregister_active_execution(&self, idempotency_key: &str) -> Result<()> {
        self.active_executions.unregister(idempotency_key)
    }

    pub(crate) fn active_execution_keys(&self) -> Result<Vec<String>> {
        self.active_executions.keys()
    }

    pub(crate) fn cancel_active_task(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
    ) -> Result<bool> {
        self.active_executions.cancel_task(task_run_id, attempt)
    }

    pub(crate) fn refresh_active_client(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
    ) -> Result<()> {
        self.active_executions.refresh_client(task_run_id, attempt)
    }

    pub(crate) fn cancel_stale_active_executions(&self) -> Result<Vec<String>> {
        self.active_executions
            .cancel_stale(self.runtime_config().remote_client_stale_ttl())
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

    pub(crate) fn admit_or_queue_resources(
        &self,
        request: ResourceRequest,
    ) -> Result<ResourceAdmissionDecision> {
        self.resource_admission.admit_or_queue(request)
    }

    pub(crate) fn wait_until_resources_admitted(
        &self,
        idempotency_key: &str,
        cancellation: &tak_runner::RunCancellation,
    ) -> Result<()> {
        self.resource_admission
            .wait_until_admitted(idempotency_key, cancellation)
    }

    pub(crate) fn release_resources(&self, idempotency_key: &str) -> Result<()> {
        self.resource_admission.release(idempotency_key)
    }

    pub(crate) fn tak_container_usage(&self) -> SharedTakContainerUsage {
        self.tak_container_usage.clone()
    }
}
