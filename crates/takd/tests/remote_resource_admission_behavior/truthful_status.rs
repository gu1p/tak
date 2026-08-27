use takd::SubmitAttemptStore;

use crate::support::remote_output::test_context;

use super::status;

#[test]
fn status_never_advertises_more_memory_than_the_host_can_supply_now() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let memory = status(&context, &store).memory.expect("memory status");
    let host_available = memory.available_bytes.expect("current host availability");
    let admittable = memory
        .tak_admission_available_bytes
        .expect("current Tak admission availability");

    assert!(
        admittable <= host_available,
        "admittable memory {admittable} exceeds current host availability {host_available}"
    );
}
