use anyhow::{Result, anyhow};

use super::fit::{
    admission_snapshot, can_fit, fits_total_capacity, promote_queued, queue_position,
    rejection_reason, reserve,
};
use super::{
    ADMISSION_CANCEL_POLL_INTERVAL, HostUsageSample, ResourceAdmissionDecision,
    ResourceAdmissionSnapshot, ResourceAdmissionState, ResourceCapacity, ResourceRequest,
    SharedResourceAdmission,
};

impl SharedResourceAdmission {
    /// Emergency admission hold: when `held`, new starts queue until cleared.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal daemon state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn set_admission_held(&self, held: bool) -> Result<()> {
        let mut state = self.lock_state()?;
        let changed = state.held != held;
        state.held = held;
        if changed && !held {
            let _ = promote_queued(&mut state);
        }
        drop(state);
        if changed {
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
        if state.queue.is_empty() && can_fit(&state, &request) {
            reserve(&mut state, request);
            return Ok(ResourceAdmissionDecision::Admitted);
        }
        state.queue.push_back(request);
        Ok(ResourceAdmissionDecision::Queued {
            queue_position: state.queue.len(),
        })
    }

    pub(crate) fn wait_until_admitted_with_positions(
        &self,
        idempotency_key: &str,
        cancellation: &tak_runner::RunCancellation,
        mut on_position: impl FnMut(usize),
    ) -> Result<()> {
        let mut state = self.lock_state()?;
        let mut last_position = None;
        loop {
            if cancellation.is_cancelled() {
                return Err(anyhow!("task cancelled"));
            }
            if state.reservations.contains_key(idempotency_key) {
                return Ok(());
            }
            let position = queue_position(&state.queue, idempotency_key)
                .ok_or_else(|| anyhow!("queued resource request disappeared"))?;
            if last_position != Some(position) {
                last_position = Some(position);
                drop(state);
                on_position(position);
                state = self.lock_state()?;
                continue;
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
        state.admitted_at.remove(idempotency_key);
        state
            .queue
            .retain(|request| request.idempotency_key != idempotency_key);
        let _ = promote_queued(&mut state);
        self.inner.changed.notify_all();
        Ok(())
    }

    pub(in crate::daemon::remote) fn update_host_usage(
        &self,
        non_tak_usage: ResourceCapacity,
        available_memory_mb: u64,
    ) -> Result<()> {
        let mut state = self.lock_state()?;
        state.host_usage = Some(HostUsageSample {
            non_tak_usage,
            available_memory_mb,
        });
        let promoted = promote_queued(&mut state);
        drop(state);
        if promoted {
            self.inner.changed.notify_all();
        }
        Ok(())
    }

    pub(crate) fn queued_jobs(&self) -> Result<Vec<ResourceRequest>> {
        Ok(self.lock_state()?.queue.iter().cloned().collect())
    }

    pub(in crate::daemon::remote) fn resource_snapshot(&self) -> Result<ResourceAdmissionSnapshot> {
        let state = self.lock_state()?;
        Ok(admission_snapshot(&state))
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, ResourceAdmissionState>> {
        self.inner
            .state
            .lock()
            .map_err(|_| anyhow!("resource admission lock poisoned"))
    }
}
