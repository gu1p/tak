use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

use super::TaskOutputObserver;

use crate::container_runtime::run_task_steps_in_container;
use crate::step_runner::{StepRunResult, run_step};

use super::remote_models::RuntimeExecutionMetadata;

/// Executes all steps in one task attempt and short-circuits on first failing step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn run_task_steps(
    task: &ResolvedTask,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StepRunResult> {
    for step in &task.steps {
        let status = run_step(
            step,
            task.timeout_s,
            workspace_root,
            runtime_env,
            &task.label,
            attempt,
            output_observer,
        )
        .await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
    })
}

pub(crate) async fn run_task_steps_with_runtime(
    task: &ResolvedTask,
    workspace_root: &Path,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StepRunResult> {
    if let Some(metadata) = runtime_metadata
        && let Some(plan) = metadata.container_plan.as_ref()
    {
        return run_task_steps_in_container(
            task,
            workspace_root,
            plan,
            Some(&metadata.env_overrides),
            attempt,
            output_observer,
        )
        .await;
    }

    run_task_steps(
        task,
        workspace_root,
        runtime_metadata.map(|metadata| &metadata.env_overrides),
        attempt,
        output_observer,
    )
    .await
}
