use tak_core::v2::RetryPolicy;

pub(super) fn delay(retry: &RetryPolicy, authored_attempt: u32) -> u64 {
    let shift = authored_attempt.saturating_sub(1).min(63);
    let delay = retry.backoff_millis.saturating_mul(1_u64 << shift);
    if retry.max_backoff_millis == 0 {
        delay
    } else {
        delay.min(retry.max_backoff_millis)
    }
}
