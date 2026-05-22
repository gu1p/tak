use tak_proto::{
    ActiveJob, ContainerResourceLimits, CpuUsage, MemoryUsage, NodeStatusResponse, QueuedJob,
};

pub(crate) fn node_status(
    node_id: &str,
    active_jobs: usize,
    queued_jobs: usize,
) -> NodeStatusResponse {
    NodeStatusResponse {
        node: None,
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 8 * 1024 * 1024 * 1024,
        }),
        storage: None,
        allocated_needs: Vec::new(),
        active_jobs: (0..active_jobs)
            .map(|index| active_job(node_id, index))
            .collect(),
        image_cache: None,
        queued_jobs: (0..queued_jobs)
            .map(|index| queued_job(node_id, index))
            .collect(),
    }
}

fn active_job(node_id: &str, index: usize) -> ActiveJob {
    ActiveJob {
        task_run_id: format!("{node_id}-active-{index}"),
        attempt: 1,
        task_label: "check".into(),
        started_at_ms: 1,
        needs: Vec::new(),
        execution_root_bytes: 0,
        runtime: Some("containerized".into()),
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("true".into()),
        resource_limits: Some(ContainerResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 512,
        }),
        execution_label: None,
    }
}

fn queued_job(node_id: &str, index: usize) -> QueuedJob {
    QueuedJob {
        task_run_id: format!("{node_id}-queued-{index}"),
        attempt: 1,
        task_label: "check".into(),
        queued_at_ms: 1,
        queue_position: u32::try_from(index + 1).expect("queue index"),
        resource_limits: Some(ContainerResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 512,
        }),
        runtime: Some("containerized".into()),
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("true".into()),
        execution_label: None,
    }
}
