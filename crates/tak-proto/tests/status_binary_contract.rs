use prost::Message;
use tak_proto::{
    ActiveJob, AggregatedNeedUsage, ContainerResourceLimits, CpuUsage, MemoryUsage, NodeInfo,
    NodeStatusResponse, QueuedJob, StorageUsage, SubmittedNeed,
};

#[test]
fn node_status_messages_round_trip_as_binary() {
    let status = NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: "builder-a".to_string(),
            healthy: true,
            transport: "direct".to_string(),
            transport_state: "ready".to_string(),
            ..Default::default()
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
            non_tak_used_cores: Some(1.0),
            tak_reserved_cores: Some(2.0),
            tak_admission_available_cores: Some(5.9),
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
            available_bytes: Some(6_144),
            non_tak_used_bytes: Some(1_024),
            tak_reserved_bytes: Some(2_048),
            tak_admission_available_bytes: Some(5_017),
        }),
        storage: Some(StorageUsage {
            path: "/tmp/takd-remote-exec".to_string(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![AggregatedNeedUsage {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 2.0,
        }],
        active_jobs: vec![ActiveJob {
            task_run_id: "task-run-1".to_string(),
            task_label: "//apps/web:build".to_string(),
            needs: vec![SubmittedNeed {
                name: "cpu".to_string(),
                scope: "machine".to_string(),
                scope_key: None,
                slots: 2.0,
            }],
            execution_root_bytes: 256,
            runtime: Some("containerized".to_string()),
            runtime_source: Some("image:alpine:3.20".to_string()),
            command: Some("make build".to_string()),
            resource_limits: Some(ContainerResourceLimits {
                cpu_cores: 2.0,
                memory_mb: 1024,
            }),
            execution_label: Some("check.build".to_string()),
            ..Default::default()
        }],
        image_cache: None,
        queued_jobs: vec![QueuedJob {
            task_run_id: "task-run-2".to_string(),
            task_label: "//apps/api:test".to_string(),
            queue_position: 1,
            resource_limits: Some(ContainerResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 512,
            }),
            runtime: Some("containerized".to_string()),
            origin: Some("task".to_string()),
            runtime_source: Some("image:alpine:3.20".to_string()),
            command: Some("make test".to_string()),
            execution_label: Some("check.test".to_string()),
            ..Default::default()
        }],
    };
    let encoded = status.encode_to_vec();
    let decoded = NodeStatusResponse::decode(encoded.as_slice()).expect("decode node status");
    let node = decoded.node.expect("node");
    assert_eq!(node.node_id, "builder-a");
    assert_eq!(node.transport_state, "ready");
    assert_eq!(decoded.active_jobs.len(), 1);
    assert_eq!(decoded.active_jobs[0].task_label, "//apps/web:build");
    let active_label = decoded.active_jobs[0].execution_label.as_deref();
    assert_eq!(active_label, Some("check.build"));
    assert_eq!(decoded.queued_jobs.len(), 1);
    assert_eq!(decoded.queued_jobs[0].queue_position, 1);
    let queued_label = decoded.queued_jobs[0].execution_label.as_deref();
    assert_eq!(queued_label, Some("check.test"));
    let cpu = decoded.cpu.expect("cpu");
    assert_eq!(cpu.tak_reserved_cores, Some(2.0));
    let memory = decoded.memory.expect("memory");
    assert_eq!(memory.tak_reserved_bytes, Some(2_048));
}
