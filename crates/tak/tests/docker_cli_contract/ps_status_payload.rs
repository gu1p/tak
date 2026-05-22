use prost::Message;
use tak_proto::{
    ActiveJob, ContainerResourceLimits, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse,
    StorageUsage,
};

pub(super) fn node_status_payload(
    node_id: &str,
    base_url: &str,
    active_jobs: Vec<ActiveJob>,
) -> Vec<u8> {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: base_url.into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
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
            path: "/tmp/takd-remote-exec".into(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs,
        image_cache: None,
        queued_jobs: vec![],
    }
    .encode_to_vec()
}

pub(super) fn active_job(
    task_label: &str,
    task_run_id: &str,
    origin: &str,
    runtime_source: &str,
    command: &str,
) -> ActiveJob {
    ActiveJob {
        task_run_id: task_run_id.into(),
        attempt: 1,
        task_label: task_label.into(),
        started_at_ms: 1_734_000_000_000,
        needs: vec![],
        execution_root_bytes: 256,
        runtime: Some("containerized".into()),
        origin: Some(origin.into()),
        runtime_source: Some(runtime_source.into()),
        command: Some(command.into()),
        resource_limits: Some(ContainerResourceLimits {
            cpu_cores: 2.0,
            memory_mb: 1024,
        }),
        execution_label: None,
    }
}
