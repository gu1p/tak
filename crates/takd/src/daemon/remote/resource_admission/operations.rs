use anyhow::{Result, anyhow};

use super::fit::{admission_snapshot, can_fit, fits_total_capacity, reserve};
use super::{
    HostUsageSample, ResourceAdmissionSnapshot, ResourceAdmissionState, ResourceCapacity,
    ResourceRequest, SharedResourceAdmission,
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
        state.held = held;
        Ok(())
    }

    pub(crate) fn admit_immediately(&self, request: ResourceRequest) -> Result<bool> {
        let mut state = self.lock_state()?;
        if state.reservations.contains_key(&request.idempotency_key) {
            return Ok(true);
        }
        if !fits_total_capacity(&state, &request) || !can_fit(&state, &request) {
            return Ok(false);
        }
        reserve(&mut state, request);
        Ok(true)
    }

    pub(crate) fn release(&self, idempotency_key: &str) -> Result<()> {
        let mut state = self.lock_state()?;
        state.reservations.remove(idempotency_key);
        state.admitted_at.remove(idempotency_key);
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
        Ok(())
    }

    pub(crate) fn reserved_jobs(&self) -> Result<Vec<ResourceRequest>> {
        Ok(self.lock_state()?.reservations.values().cloned().collect())
    }

    pub(crate) fn reservation_keys(&self) -> Result<Vec<String>> {
        Ok(self.lock_state()?.reservations.keys().cloned().collect())
    }

    pub(crate) fn has_reservations(&self) -> Result<bool> {
        Ok(!self.lock_state()?.reservations.is_empty())
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
