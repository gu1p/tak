#![cfg(test)]

use tak_core::model::{ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_proto::{ContainerResourceLimits, CpuUsage, MemoryUsage, NodeStatusResponse, QueuedJob};

use super::load_from_status;
use crate::engine::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};

#[test]
fn queued_jobs_do_not_consume_capacity_for_load_fits() {
    let status = NodeStatusResponse {
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 2,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 1024 * 1024 * 1024,
        }),
        storage: None,
        queued_jobs: vec![queued_job(), queued_job()],
        ..Default::default()
    };

    let load = load_from_status(&target(), &status);

    assert!(load.status_known);
    assert!(load.fits_requested_resources);
    assert_eq!(load.job_count, 2);
    assert_eq!(load.cpu_ratio, 0.0);
    assert_eq!(load.memory_ratio, 0.0);
}

#[test]
fn zero_or_missing_capacity_is_unknown_load() {
    let missing = NodeStatusResponse::default();
    let zero = NodeStatusResponse {
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 0,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 0,
        }),
        ..missing.clone()
    };

    assert!(!load_from_status(&target(), &missing).status_known);
    assert!(!load_from_status(&target(), &zero).status_known);
}

fn queued_job() -> QueuedJob {
    QueuedJob {
        task_run_id: "queued".into(),
        attempt: 1,
        task_label: "//:check".into(),
        queued_at_ms: 1,
        queue_position: 1,
        resource_limits: Some(resource_limits()),
        runtime: Some("containerized".into()),
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("true".into()),
        execution_label: None,
    }
}

fn resource_limits() -> ContainerResourceLimits {
    ContainerResourceLimits {
        cpu_cores: 1.0,
        memory_mb: 512,
    }
}

fn target() -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:1".into(),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".into(),
            },
            resource_limits: Some(ContainerResourceLimitsSpec {
                cpu_cores: Some(1.0),
                memory_mb: Some(512),
            }),
        }),
    }
}
