use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use tak_runner::RunCancellation;

use super::query_helpers::unix_epoch_ms;

#[derive(Clone, Default)]
pub(super) struct SharedActiveExecutions {
    inner: Arc<Mutex<ActiveExecutions>>,
}

#[derive(Default)]
struct ActiveExecutions {
    by_key: BTreeMap<String, ActiveExecution>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActiveExecutionCancelReason {
    Explicit,
    ClientStale { stale_ms: i64 },
}

pub(super) struct StaleActiveExecution {
    pub(super) idempotency_key: String,
    pub(super) stale_ms: i64,
}

struct ActiveExecution {
    task_run_id: String,
    attempt: u32,
    cancellation: RunCancellation,
    last_client_seen_ms: i64,
    cancel_reason: Option<ActiveExecutionCancelReason>,
}

impl SharedActiveExecutions {
    pub(super) fn register(
        &self,
        idempotency_key: String,
        task_run_id: &str,
        attempt: u32,
    ) -> Result<RunCancellation> {
        let cancellation = RunCancellation::new();
        self.lock()?.by_key.insert(
            idempotency_key,
            ActiveExecution {
                task_run_id: task_run_id.to_string(),
                attempt,
                cancellation: cancellation.clone(),
                last_client_seen_ms: unix_epoch_ms(),
                cancel_reason: None,
            },
        );
        Ok(cancellation)
    }

    pub(super) fn unregister(&self, idempotency_key: &str) -> Result<()> {
        self.lock()?.by_key.remove(idempotency_key);
        Ok(())
    }

    pub(super) fn keys(&self) -> Result<Vec<String>> {
        Ok(self.lock()?.by_key.keys().cloned().collect())
    }

    pub(super) fn refresh_client(&self, task_run_id: &str, attempt: Option<u32>) -> Result<()> {
        let now = unix_epoch_ms();
        let mut guard = self.lock()?;
        for execution in Self::matching_executions(&mut guard, task_run_id, attempt) {
            execution.last_client_seen_ms = now;
        }
        Ok(())
    }

    pub(super) fn cancel_task(&self, task_run_id: &str, attempt: Option<u32>) -> Result<bool> {
        let mut cancelled = false;
        let mut guard = self.lock()?;
        for execution in Self::matching_executions(&mut guard, task_run_id, attempt) {
            execution.cancellation.cancel();
            execution.cancel_reason = Some(ActiveExecutionCancelReason::Explicit);
            cancelled = true;
        }
        Ok(cancelled)
    }

    pub(super) fn cancel_stale(&self, ttl: Duration) -> Result<Vec<StaleActiveExecution>> {
        let now = unix_epoch_ms();
        let ttl_ms = i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX);
        let mut stale = Vec::new();
        let mut guard = self.lock()?;
        for (key, execution) in &mut guard.by_key {
            let stale_ms = now.saturating_sub(execution.last_client_seen_ms);
            if stale_ms >= ttl_ms {
                execution.cancellation.cancel();
                execution.cancel_reason =
                    Some(ActiveExecutionCancelReason::ClientStale { stale_ms });
                stale.push(StaleActiveExecution {
                    idempotency_key: key.clone(),
                    stale_ms,
                });
            }
        }
        Ok(stale)
    }

    pub(super) fn cancel_reason(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
    ) -> Result<Option<ActiveExecutionCancelReason>> {
        let guard = self.lock()?;
        Ok(guard
            .by_key
            .values()
            .find(|execution| {
                execution.task_run_id == task_run_id
                    && attempt.is_none_or(|attempt| execution.attempt == attempt)
            })
            .and_then(|execution| execution.cancel_reason))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ActiveExecutions>> {
        self.inner
            .lock()
            .map_err(|_| anyhow!("active remote executions lock poisoned"))
    }

    fn matching_executions<'a>(
        guard: &'a mut ActiveExecutions,
        task_run_id: &str,
        attempt: Option<u32>,
    ) -> impl Iterator<Item = &'a mut ActiveExecution> {
        guard.by_key.values_mut().filter(move |execution| {
            execution.task_run_id == task_run_id
                && attempt.is_none_or(|attempt| execution.attempt == attempt)
        })
    }
}
