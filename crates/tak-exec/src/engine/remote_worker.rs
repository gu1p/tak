use std::path::Path;

use anyhow::Result;
use tak_core::model::{
    ContainerRuntimeSourceSpec, ResolvedTask, RetryDef, TaskExecutionSpec,
    normalize_container_image_reference,
};

use super::{
    ImageCachePlan, RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec, TaskOutputObserver,
};

use super::runtime_metadata::resolve_runtime_execution_metadata_for_node_runtime_with_workspace;
use super::step_execution::run_task_steps_with_runtime;

pub async fn execute_remote_worker_steps(
    workspace_root: &Path,
    spec: &RemoteWorkerExecutionSpec,
) -> Result<RemoteWorkerExecutionResult> {
    execute_remote_worker_steps_with_output(workspace_root, spec, None).await
}

pub async fn execute_remote_worker_steps_with_output(
    workspace_root: &Path,
    spec: &RemoteWorkerExecutionSpec,
    output_observer: Option<std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<RemoteWorkerExecutionResult> {
    let task = ResolvedTask {
        label: spec.task_label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: spec.steps.clone(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: spec.timeout_s,
        context: tak_core::model::CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: TaskExecutionSpec::LocalOnly(tak_core::model::LocalSpec::default()),
        session: None,
        tags: Vec::new(),
    };

    let mut runtime_metadata = match spec.runtime.as_ref() {
        Some(runtime) => resolve_runtime_execution_metadata_for_node_runtime_with_workspace(
            &task,
            &spec.node_id,
            runtime,
            Some(workspace_root),
        )?,
        None => None,
    };
    if let Some(container_user) = spec.container_user.clone()
        && let Some(metadata) = runtime_metadata.as_mut()
        && let Some(container_plan) = metadata.container_plan.as_mut()
    {
        container_plan.container_user = Some(container_user);
    }
    if let Some(options) = spec.image_cache.clone()
        && let Some(metadata) = runtime_metadata.as_mut()
        && let Some(container_plan) = metadata.container_plan.as_mut()
    {
        container_plan.image_cache = Some(image_cache_plan(options, container_plan));
    }

    let result = run_task_steps_with_runtime(
        &task,
        workspace_root,
        runtime_metadata.as_ref(),
        spec.attempt,
        output_observer.as_ref(),
    )
    .await?;
    Ok(RemoteWorkerExecutionResult {
        success: result.success,
        exit_code: result.exit_code,
        runtime_kind: runtime_metadata
            .as_ref()
            .map(|metadata| metadata.kind.clone()),
        runtime_engine: runtime_metadata
            .as_ref()
            .and_then(|metadata| metadata.engine.clone()),
    })
}

fn image_cache_plan(
    options: crate::ImageCacheOptions,
    container_plan: &super::ContainerExecutionPlan,
) -> ImageCachePlan {
    match &container_plan.source {
        ContainerRuntimeSourceSpec::Image { image } => {
            let source_kind = normalize_container_image_reference(image)
                .map(|reference| {
                    if reference.digest_pinned {
                        "pinned"
                    } else {
                        "mutable"
                    }
                })
                .unwrap_or("mutable");
            ImageCachePlan {
                options,
                cache_key: format!("image:{image}"),
                source_kind: source_kind.to_string(),
            }
        }
        ContainerRuntimeSourceSpec::Dockerfile { .. } => ImageCachePlan {
            options,
            cache_key: format!("dockerfile:{}", container_plan.image),
            source_kind: "dockerfile".to_string(),
        },
    }
}
