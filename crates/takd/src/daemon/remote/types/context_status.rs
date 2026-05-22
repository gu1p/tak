use anyhow::{Result, anyhow};
use tak_proto::NodeStatusResponse;

use super::RemoteNodeContext;
use crate::daemon::remote::execution_root::remote_execution_root_base;
use crate::daemon::remote::status_state::{ActiveJobMetadata, SharedNodeStatusState};

impl RemoteNodeContext {
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

    pub(crate) fn update_active_job_label(
        &self,
        idempotency_key: &str,
        task_label: &str,
        execution_label: Option<String>,
    ) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.update_job_label(idempotency_key, task_label, execution_label);
        Ok(())
    }

    pub(crate) fn node_status(&self) -> Result<NodeStatusResponse> {
        let node = self.node_info()?;
        let queued_jobs = self.resource_admission.queued_jobs()?;
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.snapshot(
            &node,
            &remote_execution_root_base(self),
            self.image_cache.as_ref(),
            queued_jobs,
        )
    }

    pub(crate) fn shared_status_state(&self) -> SharedNodeStatusState {
        self.status_state.clone()
    }
}
