use anyhow::{Result, anyhow, bail};
use tak_core::model::{normalize_container_image_reference, normalize_path_ref};
use tak_core::v2::{
    ContainerMount, ContainerSource, Execution, LocalExecution, RuntimeResources, TaskRuntime,
};

use super::super::v2_wire as wire;

pub(super) fn convert_container(container: wire::Container) -> Result<TaskRuntime> {
    if container.kind != "containerized" {
        bail!("v2 container kind must be containerized");
    }
    if container.command.is_some() {
        bail!(
            "Container `command` was removed in v2; use task steps (`cmd(...)` or `script(...)`)"
        );
    }
    let source = match (container.image, container.dockerfile) {
        (Some(image), None) => ContainerSource::Image {
            image: normalize_container_image_reference(&image)?.canonical,
        },
        (None, Some(dockerfile)) => {
            let dockerfile = path(dockerfile, "dockerfile")?;
            let build_context = container
                .build_context
                .map(|value| path(value, "build_context"))
                .transpose()?
                .unwrap_or_else(|| ".".into());
            if !within(&dockerfile, &build_context) {
                bail!("v2 container dockerfile must be within build_context");
            }
            ContainerSource::Dockerfile {
                dockerfile,
                build_context,
            }
        }
        _ => bail!("v2 container requires exactly one image or dockerfile"),
    };
    let mounts = container
        .mounts
        .into_iter()
        .map(authored_mount)
        .collect::<Result<Vec<_>>>()?;
    let runtime = TaskRuntime::configured_container(
        source,
        Vec::new(),
        container.env,
        resources(container.resource_limits)?,
    )
    .map_err(anyhow::Error::msg)?;
    let TaskRuntime::Container {
        source,
        env,
        resources,
        ..
    } = runtime;
    Ok(TaskRuntime::Container {
        source,
        mounts,
        env,
        resources,
    })
}

fn authored_mount(mount: wire::ContainerMount) -> Result<ContainerMount> {
    let authored = mount.source.trim();
    let (rooted, source) = authored
        .strip_prefix("//")
        .map_or((false, authored), |source| (true, source));
    let source = if source.is_empty() { "." } else { source };
    let mut result =
        ContainerMount::new(source, mount.target, mount.read_only).map_err(anyhow::Error::msg)?;
    if rooted {
        result.source = format!("//{}", result.source.trim_start_matches("./"));
    }
    Ok(result)
}

pub(super) fn with_default_runtime(
    execution: Option<Execution>,
    runtime: Option<TaskRuntime>,
) -> Option<Execution> {
    match (execution, runtime) {
        (Some(mut execution), runtime) => {
            attach_if_missing(&mut execution, runtime);
            Some(execution)
        }
        (None, Some(runtime)) => Some(Execution::LocalOnly {
            local: LocalExecution {
                reason: String::new(),
                session: None,
                runtime: Some(runtime),
            },
        }),
        (None, None) => None,
    }
}

pub(super) fn attach_if_missing(execution: &mut Execution, runtime: Option<TaskRuntime>) {
    match execution {
        Execution::LocalOnly { local } => {
            if local.runtime.is_none() {
                local.runtime = runtime;
            }
        }
        Execution::RemoteOnly { remote } => {
            if remote.runtime.is_none() {
                remote.runtime = runtime;
            }
        }
        Execution::FirstAvailable { placements, .. } => {
            for placement in placements {
                attach_if_missing(placement, runtime.clone());
            }
        }
    }
}

fn path(value: wire::Output, field: &str) -> Result<String> {
    let wire::Output::Path { value } = value else {
        bail!("v2 container {field} must use path(...)")
    };
    Ok(normalize_path_ref("workspace", &value)
        .map_err(|error| anyhow!("invalid v2 container {field}: {error}"))?
        .path)
}

fn resources(value: Option<wire::ContainerResources>) -> Result<Option<RuntimeResources>> {
    let Some(value) = value else { return Ok(None) };
    let (Some(cpu), Some(memory_mb)) = (value.cpu_cores, value.memory_mb) else {
        bail!("v2 container resources require cpu_cores and memory_mb")
    };
    if !cpu.is_finite() || cpu <= 0.0 || memory_mb == 0 {
        bail!("v2 container resources must be positive")
    }
    let cpu_millis = (cpu * 1_000.0).round();
    if cpu_millis > u64::MAX as f64 {
        bail!("v2 container cpu resource is too large")
    }
    Ok(Some(RuntimeResources {
        cpu_millis: cpu_millis as u64,
        memory_bytes: memory_mb
            .checked_mul(1024 * 1024)
            .ok_or_else(|| anyhow!("v2 container memory resource is too large"))?,
    }))
}

fn within(path: &str, root: &str) -> bool {
    root == "."
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}
