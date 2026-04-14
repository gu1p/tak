use prost::Message;
use tak_proto::{
    ActiveJob, AggregatedNeedUsage, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse,
    StorageUsage, SubmittedNeed,
};

#[test]
fn node_status_messages_round_trip_as_binary() {
    let status = NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: "builder-a".to_string(),
            display_name: "Builder A".to_string(),
            base_url: "http://127.0.0.1:43123".to_string(),
            healthy: true,
            pools: vec!["default".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: "direct".to_string(),
            transport_state: "ready".to_string(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
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
            attempt: 1,
            task_label: "//apps/web:build".to_string(),
            started_at_ms: 1_734_000_000_000,
            needs: vec![SubmittedNeed {
                name: "cpu".to_string(),
                scope: "machine".to_string(),
                scope_key: None,
                slots: 2.0,
            }],
            execution_root_bytes: 256,
            runtime: Some("containerized".to_string()),
        }],
    };
    let encoded = status.encode_to_vec();
    let decoded = NodeStatusResponse::decode(encoded.as_slice()).expect("decode node status");
    let node = decoded.node.expect("node");
    assert_eq!(node.node_id, "builder-a");
    assert_eq!(node.transport_state, "ready");
    assert_eq!(decoded.active_jobs.len(), 1);
    assert_eq!(decoded.active_jobs[0].task_label, "//apps/web:build");
}
