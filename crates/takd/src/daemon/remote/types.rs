use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use tak_proto::NodeInfo;

use super::active_executions::SharedActiveExecutions;
use super::resource_admission::{
    HostUsageSample, ResourceAdmissionDecision, ResourceCapacity, ResourceRequest,
    SharedResourceAdmission,
};
use super::resource_policy::RemoteResourcePolicy;
use super::resource_pressure_controller::ResourcePressureSnapshot;
use super::tak_container_usage::SharedTakContainerUsage;

#[path = "types_tests.rs"]
mod tests;
use super::runtime::RemoteRuntimeConfig;
use super::runtime_state::RemoteRuntimeState;
use super::status_state::{SharedNodeStatusState, new_shared_node_status_state};

mod context_active_executions;
mod context_new;
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
    resource_policy: RemoteResourcePolicy,
    tak_container_usage: SharedTakContainerUsage,
    resource_pressure: Arc<Mutex<ResourcePressureSnapshot>>,
    runtime_state: Arc<RemoteRuntimeState>,
    runtime_services_started: Arc<AtomicBool>,
    image_cache: Option<RemoteImageCacheRuntimeConfig>,
    state_root: Option<PathBuf>,
}

impl RemoteNodeContext {
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

    pub(crate) fn replace_node_info(&self, node: NodeInfo) -> Result<()> {
        *self
            .node
            .lock()
            .map_err(|_| anyhow!("remote node lock poisoned"))? = node;
        Ok(())
    }

    pub(crate) fn claim_remote_runtime_services(&self) -> bool {
        !self.runtime_services_started.swap(true, Ordering::AcqRel)
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

    pub(crate) fn resolve_remote_resource_limits(
        &self,
        authored: Option<tak_core::model::ContainerResourceLimitsSpec>,
    ) -> tak_core::model::ContainerResourceLimitsSpec {
        self.resource_policy.resolve(authored)
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

    /// Set/clear the emergency admission hold (memory-pressure controller).
    ///
    /// ```no_run
    /// # // Reason: depends on internal resource-admission state held by the daemon.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn set_admission_held(&self, held: bool) -> Result<()> {
        self.resource_admission.set_admission_held(held)
    }

    pub(crate) fn tak_container_usage(&self) -> SharedTakContainerUsage {
        self.tak_container_usage.clone()
    }

    pub(crate) fn resource_admission(&self) -> SharedResourceAdmission {
        self.resource_admission.clone()
    }

    pub(crate) fn set_resource_pressure_snapshot(
        &self,
        snapshot: ResourcePressureSnapshot,
    ) -> Result<()> {
        *self
            .resource_pressure
            .lock()
            .map_err(|_| anyhow!("resource pressure status lock poisoned"))? = snapshot;
        Ok(())
    }

    pub(crate) fn resource_pressure_snapshot(&self) -> Result<ResourcePressureSnapshot> {
        self.resource_pressure
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| anyhow!("resource pressure status lock poisoned"))
    }
}
