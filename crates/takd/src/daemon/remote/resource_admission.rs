use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

mod fit;
mod request;
#[path = "resource_admission_reservation_tests.rs"]
mod reservation_tests;
#[path = "resource_admission_test_support.rs"]
mod test_support;
#[path = "resource_admission_tests.rs"]
mod tests;

use fit::{can_fit, fits_total_capacity, promote_queued, queue_position, rejection_reason};

pub(crate) use request::{ResourceRequest, ResourceRequestInput, proto_resource_limits};

use super::tak_container_usage::SharedTakContainerUsage;

const ADMISSION_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Copy)]
pub(super) struct ResourceCapacity {
    pub(super) cpu_cores: f64,
    pub(super) memory_mb: u64,
}

#[derive(Clone)]
pub(crate) struct SharedResourceAdmission {
    inner: Arc<ResourceAdmissionLock>,
}

struct ResourceAdmissionLock {
    state: Mutex<ResourceAdmissionState>,
    changed: Condvar,
}

struct ResourceAdmissionState {
    capacity: ResourceCapacity,
    reservations: BTreeMap<String, ResourceRequest>,
    queue: VecDeque<ResourceRequest>,
    /// Cumulative reservations may exceed raw capacity by this factor (>=1):
    /// admission is intentionally tolerant and the memory-pressure controller is
    /// the runtime backstop. Never relaxes `fits_total_capacity`.
    oversubscribe_x: u64,
    /// When the controller is in its emergency band it sets this; new starts are
    /// then queued (never admitted) until it clears. Does not evict running work.
    held: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ResourceAdmissionDecision {
    Admitted,
    Queued { queue_position: usize },
    Rejected { reason: String },
}

impl SharedResourceAdmission {
    pub(crate) fn new_detected(
        _tak_container_usage: SharedTakContainerUsage,
        oversubscribe_x: u64,
    ) -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        system.refresh_memory();
        system.refresh_cpu_all();
        let capacity = ResourceCapacity {
            cpu_cores: system.cpus().len().max(1) as f64,
            memory_mb: (system.total_memory() / 1024 / 1024).max(1),
        };
        Self {
            inner: Arc::new(ResourceAdmissionLock {
                state: Mutex::new(ResourceAdmissionState {
                    capacity,
                    reservations: BTreeMap::new(),
                    queue: VecDeque::new(),
                    oversubscribe_x: oversubscribe_x.max(1),
                    held: false,
                }),
                changed: Condvar::new(),
            }),
        }
    }

    /// Emergency admission hold: when `held`, new starts queue (never admitted)
    /// until cleared. Running reservations are untouched. Set by the
    /// memory-pressure controller's emergency band.
    ///
    /// ```no_run
    /// # // Reason: operates on internal locked admission state.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn set_admission_held(&self, held: bool) -> Result<()> {
        let mut state = self.lock_state()?;
        let changed = state.held != held;
        state.held = held;
        drop(state);
        if changed {
            // Wake admission waiters so a cleared hold promotes queued starts.
            self.inner.changed.notify_all();
        }
        Ok(())
    }

    pub(crate) fn admit_or_queue(
        &self,
        request: ResourceRequest,
    ) -> Result<ResourceAdmissionDecision> {
        let mut state = self.lock_state()?;
        if !fits_total_capacity(&state.capacity, &request) {
            return Ok(ResourceAdmissionDecision::Rejected {
                reason: rejection_reason(&state.capacity, &request),
            });
        }
        if state.reservations.contains_key(&request.idempotency_key) {
            return Ok(ResourceAdmissionDecision::Admitted);
        }
        if let Some(position) = queue_position(&state.queue, &request.idempotency_key) {
            return Ok(ResourceAdmissionDecision::Queued {
                queue_position: position,
            });
        }
        if state.queue.is_empty() && can_fit(&mut state, &request) {
            state
                .reservations
                .insert(request.idempotency_key.clone(), request);
            return Ok(ResourceAdmissionDecision::Admitted);
        }
        state.queue.push_back(request);
        Ok(ResourceAdmissionDecision::Queued {
            queue_position: state.queue.len(),
        })
    }

    pub(crate) fn wait_until_admitted(
        &self,
        idempotency_key: &str,
        cancellation: &tak_runner::RunCancellation,
    ) -> Result<()> {
        let mut state = self.lock_state()?;
        loop {
            if cancellation.is_cancelled() {
                return Err(anyhow!("task cancelled"));
            }
            promote_queued(&mut state);
            if state.reservations.contains_key(idempotency_key) {
                return Ok(());
            }
            state = self
                .inner
                .changed
                .wait_timeout(state, ADMISSION_CANCEL_POLL_INTERVAL)
                .map(|(state, _)| state)
                .map_err(|_| anyhow!("resource admission lock poisoned"))?;
        }
    }

    pub(crate) fn release(&self, idempotency_key: &str) -> Result<()> {
        let mut state = self.lock_state()?;
        state.reservations.remove(idempotency_key);
        state
            .queue
            .retain(|request| request.idempotency_key != idempotency_key);
        promote_queued(&mut state);
        self.inner.changed.notify_all();
        Ok(())
    }

    pub(crate) fn queued_jobs(&self) -> Result<Vec<ResourceRequest>> {
        let state = self.lock_state()?;
        Ok(state.queue.iter().cloned().collect())
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, ResourceAdmissionState>> {
        self.inner
            .state
            .lock()
            .map_err(|_| anyhow!("resource admission lock poisoned"))
    }
}
