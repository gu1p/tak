use tak_proto::{
    ActiveJob, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse, StorageUsage, SubmittedNeed,
};

pub(super) fn status_value(
    node_id: &str,
    base_url: &str,
    transport: &str,
    with_job: bool,
    transport_detail: &str,
) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: base_url.to_string(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: transport.into(),
            transport_state: "ready".into(),
            transport_detail: transport_detail.into(),
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
            path: "/tmp/takd-remote-exec".into(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs: active_jobs(with_job),
        image_cache: None,
    }
}

fn active_jobs(with_job: bool) -> Vec<ActiveJob> {
    if !with_job {
        return vec![];
    }
    vec![ActiveJob {
        task_run_id: "task-run-1".into(),
        attempt: 1,
        task_label: "//apps/web:build".into(),
        started_at_ms: 1_734_000_000_000,
        needs: vec![SubmittedNeed {
            name: "cpu".into(),
            scope: "machine".into(),
            scope_key: None,
            slots: 2.0,
        }],
        execution_root_bytes: 256,
        runtime: Some("containerized".into()),
    }]
}
