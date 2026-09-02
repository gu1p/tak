use anyhow::{Result, anyhow, bail};
use tak_core::model::{
    ContainerRuntimeSourceSpec, PathAnchor, RemoteRuntimeSpec, TaskExecutionSpec,
};
use tak_core::v2::{ContainerMount, ContainerSource, RuntimeResources, TaskRuntime};

pub(super) fn from_execution(execution: &TaskExecutionSpec) -> Result<Option<TaskRuntime>> {
    let runtime = match execution {
        TaskExecutionSpec::LocalOnly(local) => local.runtime.as_ref(),
        TaskExecutionSpec::RemoteOnly(remote) => remote.runtime.as_ref(),
        _ => bail!("synthetic Make task has unsupported execution policy"),
    };
    runtime.map(convert).transpose()
}

fn convert(runtime: &RemoteRuntimeSpec) -> Result<TaskRuntime> {
    let (source, mounts, env, resource_limits) = match runtime {
        RemoteRuntimeSpec::Containerized {
            source,
            resource_limits,
        } => (source, Vec::new(), Default::default(), resource_limits),
        RemoteRuntimeSpec::ContainerizedV2 {
            source,
            mounts,
            env,
            resource_limits,
            ..
        } => (
            source,
            mounts
                .iter()
                .map(|mount| {
                    ContainerMount::new(mount.source.clone(), mount.target.clone(), mount.read_only)
                        .map_err(anyhow::Error::msg)
                })
                .collect::<Result<Vec<_>>>()?,
            env.clone(),
            resource_limits,
        ),
    };
    let source = match source {
        ContainerRuntimeSourceSpec::Image { image } => ContainerSource::Image {
            image: image.clone(),
        },
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        } => ContainerSource::Dockerfile {
            dockerfile: workspace_path(dockerfile, "Dockerfile")?,
            build_context: workspace_path(build_context, "build context")?,
        },
    };
    TaskRuntime::configured_container(
        source,
        mounts,
        env,
        resource_limits.as_ref().map(resources).transpose()?,
    )
    .map_err(anyhow::Error::msg)
}

fn workspace_path(path: &tak_core::model::PathRef, field: &str) -> Result<String> {
    if path.anchor != PathAnchor::Workspace {
        bail!("Make container {field} must be workspace-relative")
    }
    Ok(path.path.clone())
}

fn resources(value: &tak_core::model::ContainerResourceLimitsSpec) -> Result<RuntimeResources> {
    let (Some(cpu), Some(memory_mb)) = (value.cpu_cores, value.memory_mb) else {
        bail!("Make container resources require cpu_cores and memory_mb")
    };
    let cpu_millis = (cpu * 1_000.0).round();
    if !cpu.is_finite() || cpu <= 0.0 || cpu_millis > u64::MAX as f64 || memory_mb == 0 {
        bail!("Make container resources must be positive")
    }
    Ok(RuntimeResources {
        cpu_millis: cpu_millis as u64,
        memory_bytes: memory_mb
            .checked_mul(1024 * 1024)
            .ok_or_else(|| anyhow!("Make container memory resource is too large"))?,
    })
}
