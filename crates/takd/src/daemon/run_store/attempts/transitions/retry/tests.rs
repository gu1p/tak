use std::num::NonZeroU32;

use tak_core::v2::{RetryJitter, RetryPolicy};

use super::delay;

#[test]
fn full_jitter_is_bounded_and_stable_for_one_attempt_fence() {
    let policy = RetryPolicy {
        max_attempts: NonZeroU32::new(3).unwrap(),
        on_exit: vec![],
        backoff_millis: 1_000,
        max_backoff_millis: 4_000,
        jitter: RetryJitter::Full,
    };
    let first = delay(&policy, 3, "fence-a");
    assert_eq!(first, delay(&policy, 3, "fence-a"));
    assert!(first <= 4_000);
    assert_ne!(first, delay(&policy, 3, "fence-b"));
}
