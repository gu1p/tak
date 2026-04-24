use std::path::Path;

use anyhow::Result;
use tak_core::model::{ResolvedTask, RetryDef, TaskExecutionSpec};

use super::{RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec, TaskOutputObserver};

use super::runtime_metadata::resolve_runtime_execution_metadata_for_node_runtime;
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

    let runtime_metadata = match spec.runtime.as_ref() {
        Some(runtime) => resolve_runtime_execution_metadata_for_node_runtime(
            &task,
            &spec.node_id,
            Some(runtime),
        )?,
        None => None,
    };

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
