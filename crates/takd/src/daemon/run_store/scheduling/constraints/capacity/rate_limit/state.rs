use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::daemon::run_store::scheduling::constraints::capacity) struct BucketState {
    pub(in crate::daemon::run_store::scheduling::constraints::capacity) available_micros: u64,
    pub(in crate::daemon::run_store::scheduling::constraints::capacity) refilled_at_ms: u64,
}

impl BucketState {
    pub(in crate::daemon::run_store::scheduling::constraints::capacity) const fn new(
        available_micros: u64,
        refilled_at_ms: u64,
    ) -> Self {
        Self {
            available_micros,
            refilled_at_ms,
        }
    }

    pub(in crate::daemon::run_store::scheduling::constraints::capacity) const fn full(
        capacity_micros: u64,
        now_ms: u64,
    ) -> Self {
        Self::new(capacity_micros, now_ms)
    }
}

pub(in crate::daemon::run_store::scheduling::constraints::capacity) fn refill(
    state: BucketState,
    capacity_micros: u64,
    refill_millis_per_second: u64,
    now_ms: u64,
) -> Result<BucketState> {
    if capacity_micros == 0
        || refill_millis_per_second == 0
        || state.available_micros > capacity_micros
    {
        bail!("invalid persisted token-bucket state")
    }
    let effective_now = now_ms.max(state.refilled_at_ms);
    let elapsed = effective_now - state.refilled_at_ms;
    let gained = u128::from(elapsed) * u128::from(refill_millis_per_second);
    let available = (u128::from(state.available_micros) + gained).min(u128::from(capacity_micros));
    Ok(BucketState::new(available as u64, effective_now))
}
