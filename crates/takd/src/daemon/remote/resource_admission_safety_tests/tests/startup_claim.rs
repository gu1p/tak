use super::*;

// A global host sample cannot prove that a newly admitted task was observed.

#[test]
fn global_sample_does_not_replace_startup_claim_before_task_observation() {
    let usage = SharedTakContainerUsage::default();
    let admission = SharedResourceAdmission::new_with_elastic_startup(
        usage.clone(),
        capacity(4.0, 4096),
        1,
        capacity(1.0, 1024),
    );
    admission
        .admit_or_queue(elastic_request("elastic"))
        .expect("elastic admission");
    admission.age_admission_for_tests("elastic", Duration::from_secs(6));
    usage.set_task_snapshots_for_tests(&[]);

    let unobserved = admission
        .resource_snapshot(capacity(0.0, 0), u64::MAX)
        .expect("unobserved snapshot");

    assert_eq!(unobserved.pending_startup, capacity(1.0, 1024));

    usage.set_task_snapshots_for_tests(&[("elastic", 0.0, 0)]);
    let observed = admission
        .resource_snapshot(capacity(0.0, 0), u64::MAX)
        .expect("observed snapshot");

    assert_eq!(observed.pending_startup, capacity(0.0, 0));
}
