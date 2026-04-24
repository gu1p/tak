use std::path::Path;

use tak_core::model::{
    ContainerRuntimeSourceSpec, PolicyDecisionSpec, RemoteRuntimeSpec, ResolvedTask,
    TaskExecutionSpec, normalize_container_image_reference, normalize_path_ref,
};

use super::*;

pub(super) fn explicit_container_runtime_override(
    container_image: Option<&str>,
    container_dockerfile: Option<&str>,
    container_build_context: Option<&str>,
) -> Result<Option<RemoteRuntimeSpec>> {
    if let Some(image) = container_image {
        let image = normalize_container_image_reference(image)
            .map_err(|err| anyhow!("invalid --container-image: {err}"))?
            .canonical;
        return Ok(Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
        }));
    }

    let Some(dockerfile) = container_dockerfile else {
        return Ok(None);
    };

    let dockerfile = normalize_path_ref("workspace", dockerfile)
        .map_err(|err| anyhow!("invalid --container-dockerfile: {err}"))?;
    let build_context_value = container_build_context
        .map(str::to_string)
        .unwrap_or_else(|| infer_build_context_from_dockerfile(dockerfile.path.as_str()));
    let build_context = normalize_path_ref("workspace", &build_context_value)
        .map_err(|err| anyhow!("invalid --container-build-context: {err}"))?;
    if !path_ref_within(&dockerfile, &build_context) {
        bail!("--container-dockerfile must be within --container-build-context");
    }

    Ok(Some(RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        },
    }))
}

#[allow(dead_code)]
pub(super) fn resolve_container_runtime_for_task(
    task: &ResolvedTask,
    explicit_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<RemoteRuntimeSpec> {
    if let Some(runtime) = explicit_runtime {
        return Ok(runtime.clone());
    }
    if let Some(runtime) = declared_container_runtime(&task.execution) {
        return Ok(runtime);
    }
    if let Some(runtime) = task.container_runtime.clone() {
        return Ok(runtime);
    }

    bail!(
        "task {} requires --container-image, --container-dockerfile, or TASKS.py defaults.container_runtime when using --container",
        canonical_label(&task.label)
    )
}

pub(super) fn declared_container_runtime(
    execution: &TaskExecutionSpec,
) -> Option<RemoteRuntimeSpec> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => local.runtime.clone(),
        TaskExecutionSpec::RemoteOnly(remote) => remote.runtime.clone(),
        TaskExecutionSpec::ByCustomPolicy {
            decision:
                Some(PolicyDecisionSpec::Local {
                    local: Some(local), ..
                }),
            ..
        } => local.runtime.clone(),
        TaskExecutionSpec::ByCustomPolicy {
            decision: Some(PolicyDecisionSpec::Remote { remote, .. }),
            ..
        } => remote.runtime.clone(),
        TaskExecutionSpec::ByCustomPolicy { .. } | TaskExecutionSpec::UseSession { .. } => None,
    }
}

fn infer_build_context_from_dockerfile(dockerfile: &str) -> String {
    Path::new(dockerfile)
        .parent()
        .and_then(|path| {
            let value = path.to_string_lossy();
            (!value.is_empty()).then_some(value.into_owned())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn path_ref_within(path: &tak_core::model::PathRef, root: &tak_core::model::PathRef) -> bool {
    if path.anchor != root.anchor {
        return false;
    }
    if root.path == "." {
        return true;
    }
    if path.path == root.path {
        return true;
    }
    path.path
        .strip_prefix(&root.path)
        .is_some_and(|suffix| suffix.starts_with('/'))
}
