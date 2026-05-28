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

struct ActiveExecution {
    task_run_id: String,
    attempt: u32,
    cancellation: RunCancellation,
    last_client_seen_ms: i64,
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
            cancelled = true;
        }
        Ok(cancelled)
    }

    pub(super) fn cancel_stale(&self, ttl: Duration) -> Result<Vec<String>> {
        let now = unix_epoch_ms();
        let ttl_ms = i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX);
        let mut keys = Vec::new();
        let guard = self.lock()?;
        for (key, execution) in &guard.by_key {
            if now.saturating_sub(execution.last_client_seen_ms) >= ttl_ms {
                execution.cancellation.cancel();
                keys.push(key.clone());
            }
        }
        Ok(keys)
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
