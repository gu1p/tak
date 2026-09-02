use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::{RunCancellation, TaskOutputObserver};

use crate::container_runtime::run_task_steps_in_container;
use crate::step_runner::{StepRunContext, StepRunResult, run_step};
use crate::worker_runtime::RuntimeExecutionMetadata;

/// Executes all steps in one task attempt and short-circuits on first failing step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_task_steps(
    task: &ResolvedTask,
    workspace_root: &Path,
    base_environment: Option<&BTreeMap<String, String>>,
    clear_environment: bool,
    runtime_env: Option<&BTreeMap<String, String>>,
    attempt: u32,
    task_run_id: &str,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    cancellation: &RunCancellation,
) -> Result<StepRunResult> {
    for step in &task.steps {
        let status = run_step(
            step,
            task.timeout_s,
            StepRunContext {
                workspace_root,
                base_environment,
                clear_environment,
                runtime_env,
                task_label: &task.label,
                attempt,
                task_run_id,
                output_observer,
                cancellation,
                container_identity: None,
                container_node_id: None,
            },
        )
        .await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
        container_oom_killed: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_task_steps_with_runtime(
    task: &ResolvedTask,
    workspace_root: &Path,
    base_environment: Option<&BTreeMap<String, String>>,
    clear_environment: bool,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
    attempt: u32,
    task_run_id: &str,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    cancellation: &RunCancellation,
) -> Result<StepRunResult> {
    if let Some(metadata) = runtime_metadata
        && let Some(plan) = metadata.container_plan.as_ref()
    {
        return run_task_steps_in_container(
            task,
            plan,
            StepRunContext {
                workspace_root,
                base_environment,
                clear_environment,
                runtime_env: Some(&metadata.env_overrides),
                task_label: &task.label,
                attempt,
                task_run_id,
                output_observer,
                cancellation,
                container_identity: metadata.container_identity.as_ref(),
                container_node_id: Some(&metadata.node_id),
            },
        )
        .await;
    }

    run_task_steps(
        task,
        workspace_root,
        base_environment,
        clear_environment,
        runtime_metadata.map(|metadata| &metadata.env_overrides),
        attempt,
        task_run_id,
        output_observer,
        cancellation,
    )
    .await
}
