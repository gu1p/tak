#![cfg(test)]

use tak_core::model::{ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_proto::{ContainerResourceLimits, QueuedJob};

use crate::engine::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};

pub(super) fn queued_job() -> QueuedJob {
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

pub(super) fn target() -> StrictRemoteTarget {
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
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}
