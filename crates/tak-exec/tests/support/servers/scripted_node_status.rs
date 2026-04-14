use tak_proto::{ActiveJob, CpuUsage, ErrorResponse, MemoryUsage, NodeInfo, NodeStatusResponse};

pub(super) fn node_info(node_id: &str, port: u16) -> NodeInfo {
    NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
}

pub(super) fn node_status(node_id: &str, port: u16) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(node_info(node_id, port)),
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
        }),
        storage: None,
        allocated_needs: Vec::new(),
        active_jobs: vec![ActiveJob {
            task_run_id: "task-run-1".into(),
            attempt: 1,
            task_label: "//:delayed_terminal_events".into(),
            started_at_ms: 1,
            needs: Vec::new(),
            execution_root_bytes: 256,
            runtime: Some("containerized".into()),
        }],
    }
}

pub(super) fn status_unavailable() -> ErrorResponse {
    ErrorResponse {
        message: "status_unavailable".into(),
    }
}
