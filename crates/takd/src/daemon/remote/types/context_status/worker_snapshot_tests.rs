use std::num::NonZeroU32;

use tak_core::model::ContainerResourceLimitsSpec;
use tak_proto::worker_v2::PROTOCOL_VERSION;

use super::RemoteNodeContext;
use crate::daemon::remote::resource_admission::ResourceRequest;

#[test]
fn worker_snapshot_projects_live_node_resources_without_v1_fields() {
    let context = RemoteNodeContext::isolated_for_test();
    let snapshot = context.worker_v2_snapshot().unwrap();

    assert_eq!(snapshot.protocol_version, PROTOCOL_VERSION);
    assert_eq!(snapshot.node_id, "builder-a");
    assert!(snapshot.capacity.cpu_millis > 0);
    assert!(snapshot.capacity.memory_bytes > 0);
    assert!(snapshot.capacity.execution_slots > 0);
    assert!(snapshot.usage.cpu_millis <= snapshot.capacity.cpu_millis);
    assert!(snapshot.usage.memory_bytes <= snapshot.capacity.memory_bytes);
}

#[test]
fn worker_snapshot_projects_exact_admitted_v2_claims() {
    let context = RemoteNodeContext::isolated_for_test();
    let request = ResourceRequest {
        idempotency_key: "v2:fence".into(),
        task_run_id: "run/job".into(),
        attempt: 1,
        task_label: "//:check".into(),
        queued_at_ms: 1,
        resource_limits: ContainerResourceLimitsSpec {
            cpu_cores: Some(1.25),
            memory_mb: Some(3),
        },
        runtime: None,
        origin: None,
        runtime_source: None,
        command: None,
        execution_label: None,
        execution_slots: NonZeroU32::new(2).unwrap(),
    };
    assert!(
        context
            .resource_admission()
            .admit_immediately(request)
            .unwrap()
    );

    let snapshot = context.worker_v2_snapshot().unwrap();
    assert_eq!(snapshot.usage.cpu_millis, 1_250);
    assert_eq!(snapshot.usage.memory_bytes, 3 * 1024 * 1024);
    assert_eq!(snapshot.usage.execution_slots, 2);
    let status = context.node_status().unwrap();
    assert_eq!(status.active_jobs.len(), 1);
    assert_eq!(status.active_jobs[0].task_run_id, "run/job");
    assert_eq!(status.active_jobs[0].task_label, "//:check");
}
