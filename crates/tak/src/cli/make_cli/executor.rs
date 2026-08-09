use std::sync::Arc;

use anyhow::{Result, anyhow};
use tak_exec::{RunOptions, run_resolved_task};
use tak_make::{
    GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakeRunOutcome, RunMakeError,
};

use crate::cli::command_model::MakeArgs;
use crate::cli::run_output::StdStreamOutputObserver;
use crate::cli::run_overrides::{
    RunExecutionOverrideArgs, apply_run_execution_overrides, warn_redundant_remote_container_flag,
};

use super::task::make_workspace;

pub(super) struct TakGoalExecutor<'a> {
    args: &'a MakeArgs,
}

impl<'a> TakGoalExecutor<'a> {
    pub(super) fn new(args: &'a MakeArgs) -> Self {
        Self { args }
    }
}

impl GoalExecutor for TakGoalExecutor<'_> {
    fn execute(&self, request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
        Box::pin(async move {
            execute_make_goal(request, self.args)
                .await
                .map_err(|error| RunMakeError::execution(error.to_string()))
        })
    }
}

async fn execute_make_goal(
    request: GoalExecutionRequest,
    args: &MakeArgs,
) -> Result<MakeRunOutcome> {
    let (spec, label) = make_workspace(request)?;
    warn_redundant_container_flag(args);
    let spec = apply_run_execution_overrides(
        &spec,
        std::slice::from_ref(&label),
        run_override_args(args),
    )?;
    let task = spec
        .tasks
        .get(&label)
        .ok_or_else(|| anyhow!("missing synthetic Make task"))?;
    let result = run_resolved_task(
        task,
        &spec.root,
        &RunOptions {
            output_observer: Some(Arc::new(StdStreamOutputObserver::default())),
            ..RunOptions::default()
        },
    )
    .await?;

    Ok(MakeRunOutcome {
        exit_code: task_exit_code(result.success, result.exit_code),
    })
}

fn run_override_args(args: &MakeArgs) -> RunExecutionOverrideArgs<'_> {
    RunExecutionOverrideArgs {
        local: args.local,
        local_no_container: args.local_no_container,
        remote: args.remote,
        container: args.container,
        container_image: args.container_image.as_deref(),
        container_dockerfile: args.container_dockerfile.as_deref(),
        container_build_context: args.container_build_context.as_deref(),
    }
}

fn warn_redundant_container_flag(args: &MakeArgs) {
    if warn_redundant_remote_container_flag(args.remote, args.container) {
        eprintln!(
            "warning: --container is redundant with --remote; remote execution already implies a container"
        );
    }
}

fn task_exit_code(success: bool, code: Option<i32>) -> i32 {
    if success {
        return 0;
    }
    code.unwrap_or(1).clamp(1, u8::MAX as i32)
}
