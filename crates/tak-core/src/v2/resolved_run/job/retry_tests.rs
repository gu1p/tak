use std::num::NonZeroU32;

use super::{RetryJitter, RetryPolicy};

#[test]
fn exit_filters_and_jitter_are_explicit_durable_retry_state() {
    let policy = RetryPolicy {
        max_attempts: NonZeroU32::new(3).unwrap(),
        on_exit: vec![7, 9],
        backoff_millis: 250,
        max_backoff_millis: 4_000,
        jitter: RetryJitter::Full,
    };

    assert!(policy.allows_exit(Some(7)));
    assert!(!policy.allows_exit(Some(2)));
    assert!(!policy.allows_exit(None));
    assert_eq!(policy.jitter, RetryJitter::Full);

    let any_failure = RetryPolicy {
        on_exit: vec![],
        ..policy
    };
    assert!(any_failure.allows_exit(Some(2)));
    assert!(any_failure.allows_exit(None));
}
