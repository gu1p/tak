use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::RunOptions;
use crate::engine::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use crate::engine::fused_cascade::FusedCascade;
use crate::engine::remote_models::{RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement};
use crate::engine::step_execution::run_task_steps_with_runtime;
use crate::retry::{retry_backoff_delay, should_retry};
use crate::step_runner::StepRunResult;

pub(super) struct LocalFusedAttemptContext<'a> {
    pub(super) cascade: &'a FusedCascade,
    pub(super) workspace_root: &'a Path,
    pub(super) options: &'a RunOptions,
    pub(super) task_run_id: &'a str,
    pub(super) placement: &'a TaskPlacement,
    pub(super) runtime_metadata: Option<&'a RuntimeExecutionMetadata>,
    pub(super) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(super) run_root: &'a Path,
}

pub(super) async fn run_local_fused_attempt(
    context: LocalFusedAttemptContext<'_>,
) -> Result<(u32, AttemptExecutionOutcome)> {
    run_local_fused_members(&context).await
}

async fn run_local_fused_members(
    context: &LocalFusedAttemptContext<'_>,
) -> Result<(u32, AttemptExecutionOutcome)> {
    let mut attempts = 1;
    for member in &context.cascade.members {
        let result = run_member_with_retries(
            member,
            context.options,
            context.task_run_id,
            context.runtime_metadata,
            context.run_root,
        )
        .await?;
        attempts = attempts.max(result.attempts);
        if !result.status.success {
            return Ok((
                attempts,
                failed_member_outcome(member, result.status.exit_code),
            ));
        }
    }
    collect_fused_outputs(context, attempts).await
}

struct MemberRunResult {
    attempts: u32,
    status: StepRunResult,
}

async fn run_member_with_retries(
    member: &ResolvedTask,
    options: &RunOptions,
    task_run_id: &str,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
    run_root: &Path,
) -> Result<MemberRunResult> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let status = run_task_steps_with_runtime(
            member,
            run_root,
            runtime_metadata,
            attempt,
            task_run_id,
            options.output_observer.as_ref(),
            &options.cancellation,
        )
        .await?;
        if status.success || !can_retry(member, attempt, status.exit_code) {
            return Ok(MemberRunResult {
                attempts: attempt,
                status,
            });
        }
        wait_before_retry(member, attempt).await;
    }
}

fn can_retry(task: &ResolvedTask, attempt: u32, exit_code: Option<i32>) -> bool {
    attempt < task.retry.attempts.max(1) && should_retry(exit_code, &task.retry.on_exit)
}

async fn wait_before_retry(task: &ResolvedTask, attempt: u32) {
    let wait = retry_backoff_delay(&task.retry.backoff, attempt);
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

async fn collect_fused_outputs(
    context: &LocalFusedAttemptContext<'_>,
    attempt: u32,
) -> Result<(u32, AttemptExecutionOutcome)> {
    let context = AttemptExecutionContext {
        task: &context.cascade.task,
        workspace_root: context.workspace_root,
        run_root: context.run_root,
        placement: context.placement,
        runtime_metadata: context.runtime_metadata,
        remote_workspace: context.remote_workspace,
        task_run_id: context.task_run_id,
        attempt,
        output_observer: context.options.output_observer.as_ref(),
        cancellation: &context.options.cancellation,
    };
    execute_task_attempt(&context)
        .await
        .map(|outcome| (attempt, outcome))
}

fn failed_member_outcome(member: &ResolvedTask, exit_code: Option<i32>) -> AttemptExecutionOutcome {
    AttemptExecutionOutcome {
        attempt_success: false,
        last_exit_code: exit_code,
        failure_detail: Some(format!("fused member {} failed", member.label)),
        synced_outputs: Vec::new(),
        remote_runtime_kind: None,
        remote_runtime_engine: None,
        remote_logs: Vec::new(),
    }
}
