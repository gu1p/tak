use super::super::{build_submit_idempotency_key, sanitize_submit_idempotency_key};
use super::CLEANUP_TOMBSTONE_PREFIX;

#[test]
fn tombstone_namespace_cannot_be_authored_by_a_submit_id() {
    let submit_key =
        build_submit_idempotency_key(CLEANUP_TOMBSTONE_PREFIX, Some(1)).expect("submit key");
    let storage_name = sanitize_submit_idempotency_key(&submit_key);

    assert!(
        !storage_name.starts_with(CLEANUP_TOMBSTONE_PREFIX),
        "live submit storage was classified as a cleanup tombstone: {storage_name}"
    );
}
