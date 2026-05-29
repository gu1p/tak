use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

mod request;
#[path = "resource_admission_reservation_tests.rs"]
mod reservation_tests;
#[path = "resource_admission_test_support.rs"]
mod test_support;
#[path = "resource_admission_tests.rs"]
mod tests;

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
}

#[derive(Debug, Clone)]
pub(crate) enum ResourceAdmissionDecision {
    Admitted,
    Queued { queue_position: usize },
}

impl SharedResourceAdmission {
    pub(crate) fn new_detected(_tak_container_usage: SharedTakContainerUsage) -> Self {
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
                }),
                changed: Condvar::new(),
            }),
        }
    }

    pub(crate) fn admit_or_queue(
        &self,
        request: ResourceRequest,
    ) -> Result<ResourceAdmissionDecision> {
        let mut state = self.lock_state()?;
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

fn promote_queued(state: &mut ResourceAdmissionState) {
    loop {
        let Some(next) = state.queue.front().cloned() else {
            return;
        };
        if !can_fit(state, &next) {
            return;
        }
        let next = state.queue.pop_front().expect("queued request");
        state
            .reservations
            .insert(next.idempotency_key.clone(), next);
    }
}

fn can_fit(state: &mut ResourceAdmissionState, request: &ResourceRequest) -> bool {
    let used = reserved_totals(state);
    let requested_cpu = request.resource_limits.cpu_cores.unwrap_or(0.0);
    let requested_memory = request.resource_limits.memory_mb.unwrap_or(0);
    used.cpu_cores + requested_cpu <= state.capacity.cpu_cores
        && used.memory_mb.saturating_add(requested_memory) <= state.capacity.memory_mb
}

fn reserved_totals(state: &ResourceAdmissionState) -> ResourceCapacity {
    state.reservations.values().fold(
        ResourceCapacity {
            cpu_cores: 0.0,
            memory_mb: 0,
        },
        |mut totals, request| {
            totals.cpu_cores += request.resource_limits.cpu_cores.unwrap_or(0.0);
            totals.memory_mb = totals
                .memory_mb
                .saturating_add(request.resource_limits.memory_mb.unwrap_or(0));
            totals
        },
    )
}
