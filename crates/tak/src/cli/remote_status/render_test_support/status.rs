use tak_proto::{
    ActiveJob, ContainerResourceLimits, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse,
    StorageUsage, SubmittedNeed,
};

pub(super) fn status(node_id: &str, transport_state: &str, with_job: bool) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.to_string(),
            display_name: node_id.to_string(),
            base_url: format!("http://{node_id}.example"),
            healthy: true,
            pools: vec!["default".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: "direct".to_string(),
            transport_state: transport_state.to_string(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
            ..Default::default()
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
            ..Default::default()
        }),
        storage: Some(StorageUsage {
            path: "/tmp/takd-remote-exec".to_string(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs: active_jobs(with_job),
        image_cache: None,
        queued_jobs: vec![],
        resource_envelope: None,
        resource_pressure: None,
    }
}

fn active_jobs(with_job: bool) -> Vec<ActiveJob> {
    if !with_job {
        return Vec::new();
    }
    vec![ActiveJob {
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
        origin: Some("task".to_string()),
        runtime_source: Some("image:alpine:3.20".to_string()),
        command: Some("make build".to_string()),
        resource_limits: Some(ContainerResourceLimits {
            cpu_cores: 2.0,
            memory_mb: 1024,
        }),
        execution_label: Some("check.build".to_string()),
    }]
}
