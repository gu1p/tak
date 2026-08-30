use super::rate_limit::{BucketState, refill};

#[test]
fn fixed_point_refill_carries_sub_millislot_progress_exactly() {
    let empty = BucketState::new(0, 0);
    let almost = refill(empty, 1_000_000, 1, 999).unwrap();
    assert_eq!(almost, BucketState::new(999, 999));
    let one_millislot = refill(almost, 1_000_000, 1, 1_000).unwrap();
    assert_eq!(one_millislot, BucketState::new(1_000, 1_000));
}

#[test]
fn refill_clamps_at_burst_without_future_credit_or_backward_minting() {
    let near_full = BucketState::new(999_900, 10);
    let full = refill(near_full, 1_000_000, 1_000, 11).unwrap();
    assert_eq!(full, BucketState::new(1_000_000, 11));
    assert_eq!(refill(full, 1_000_000, 1_000, 5).unwrap(), full);

    let huge = BucketState::new(0, 0);
    let capacity = u64::from(u32::MAX) * 1_000_000;
    assert_eq!(
        refill(huge, capacity, i64::MAX as u64, u64::MAX)
            .unwrap()
            .available_micros,
        capacity
    );
}

#[test]
fn persisted_bucket_state_rejects_malformed_values() {
    let malformed = BucketState::new(1_001, 0);
    assert!(refill(malformed, 1_000, 1, 1).is_err());
}
