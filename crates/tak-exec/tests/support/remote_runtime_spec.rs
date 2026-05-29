use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::RemoteWorkerExecutionSpec;

use super::{shell_step, worker_spec};

/// Build a worker spec that runs `command` in a containerized `alpine:3.20`
/// runtime on `builder-a` (no resource limits).
pub fn alpine_spec(name: &str, command: &str) -> RemoteWorkerExecutionSpec {
    worker_spec(
        name,
        vec![shell_step(command)],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
            resource_limits: None,
        }),
        "builder-a",
    )
}
