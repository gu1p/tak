// Reconnect backoff math for peers whose heartbeat has failed: an exponential
// delay capped at a ceiling, with per-peer jitter so a fleet does not retry in
// lockstep.

const RECONNECT_BACKOFF_INITIAL_MS: i64 = 1_000;
const RECONNECT_BACKOFF_MAX_MS: i64 = 60_000;

pub(super) fn next_retry_due_ms(node_id: &str, attempts: u32, now_ms: i64) -> i64 {
    now_ms.saturating_add(backoff_delay_ms(node_id, attempts))
}

fn backoff_delay_ms(node_id: &str, attempts: u32) -> i64 {
    let exponent = attempts.saturating_sub(1).min(16);
    let base = RECONNECT_BACKOFF_INITIAL_MS
        .saturating_mul(1_i64 << exponent)
        .min(RECONNECT_BACKOFF_MAX_MS);
    apply_jitter(base, node_id, attempts)
}

fn apply_jitter(base_ms: i64, node_id: &str, attempts: u32) -> i64 {
    let mut hash = u64::from(attempts);
    for byte in node_id.as_bytes() {
        hash = hash
            .wrapping_mul(1099511628211)
            .wrapping_add(u64::from(*byte));
    }
    let jitter_percent = i64::try_from(hash % 41).unwrap_or(0) - 20;
    base_ms.saturating_mul(100 + jitter_percent) / 100
}
