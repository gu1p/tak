use takd::SubmitAttemptStore;

use crate::support::remote_output::test_context;

use super::status;

#[test]
fn ignored_host_mode_advertises_its_usable_workload_capacity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let snapshot = status(&context, &store);
    let envelope = snapshot.resource_envelope.expect("resource envelope");

    assert!(envelope.admittable_cpu_cores > 0.0);
    assert!(envelope.admittable_memory_bytes > 0);
    assert_eq!(
        envelope.admittable_memory_bytes,
        envelope.workload_memory_bytes
    );
}
