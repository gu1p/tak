use sha2::{Digest, Sha256};
use tak_core::v2::{RetryJitter, RetryPolicy};

#[cfg(test)]
mod tests;

pub(super) fn delay(retry: &RetryPolicy, authored_attempt: u32, fence: &str) -> u64 {
    let shift = authored_attempt.saturating_sub(1).min(63);
    let delay = retry.backoff_millis.saturating_mul(1_u64 << shift);
    let bounded = if retry.max_backoff_millis == 0 {
        delay
    } else {
        delay.min(retry.max_backoff_millis)
    };
    if retry.jitter == RetryJitter::None || bounded == 0 {
        return bounded;
    }
    let digest = Sha256::digest(fence.as_bytes());
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    let sampled = u64::from_be_bytes(prefix);
    sampled % bounded.saturating_add(1)
}
