use anyhow::Result;

use super::RemoteNodeContext;
use crate::daemon::remote::active_executions::{ActiveExecutionCancelReason, StaleActiveExecution};

impl RemoteNodeContext {
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

    pub(in crate::daemon::remote) fn try_when_no_active_executions<T>(
        &self,
        operation: impl FnOnce() -> Result<Option<T>>,
    ) -> Result<Option<T>> {
        self.active_executions.try_when_idle(operation)
    }

    pub(crate) fn unregister_active_execution_after_locked<T>(
        &self,
        idempotency_key: &str,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        self.active_executions
            .unregister_after_locked(idempotency_key, operation)
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

    pub(in crate::daemon::remote) fn cancel_stale_active_executions(
        &self,
    ) -> Result<Vec<StaleActiveExecution>> {
        self.active_executions
            .cancel_stale(self.runtime_config().remote_client_stale_ttl())
    }

    pub(in crate::daemon::remote) fn active_execution_cancel_reason(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
    ) -> Result<Option<ActiveExecutionCancelReason>> {
        self.active_executions.cancel_reason(task_run_id, attempt)
    }
}
