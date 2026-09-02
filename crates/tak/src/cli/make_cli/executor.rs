use anyhow::{Context, Result};
use tak_make::{
    GoalAnnotations, GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakeExecutionPlan,
    MakeRunOutcome, ParallelOutputMode, RunMakeError,
};

use crate::cli::command_model::{MakeArgs, MakeParallelOutputArg};
use crate::cli::run_overrides::{
    RunExecutionOverrideArgs, apply_run_execution_overrides, warn_redundant_remote_container_flag,
};

use super::output::ParallelMakeOutputObserver;
use super::resolved;
use super::task::{make_workspace, parallel_make_workspace};

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

    fn execute_plan(&self, plan: MakeExecutionPlan) -> GoalExecutionFuture<'_> {
        Box::pin(async move {
            execute_parallel_make(plan, self.args)
                .await
                .map_err(|error| RunMakeError::execution(error.to_string()))
        })
    }
}

async fn execute_make_goal(
    request: GoalExecutionRequest,
    args: &MakeArgs,
) -> Result<MakeRunOutcome> {
    let override_args = run_override_args(args);
    let implicit_local_host =
        request.annotations == GoalAnnotations::default() && !override_args.is_configured();
    let (spec, label) = make_workspace(request)?;
    warn_redundant_container_flag(args);
    let spec = apply_run_execution_overrides(&spec, std::slice::from_ref(&label), override_args)?;
    if implicit_local_host {
        print_implicit_local_host_notice(&args.goal);
    }
    let exit_code = submit(&spec, &[label], 1, false, args, None).await?;
    Ok(MakeRunOutcome { exit_code })
}

async fn execute_parallel_make(plan: MakeExecutionPlan, args: &MakeArgs) -> Result<MakeRunOutcome> {
    let override_args = run_override_args(args);
    let implicit_local_host = plan
        .goals
        .iter()
        .all(|goal| goal.annotations == GoalAnnotations::default())
        && !override_args.is_configured();
    let root_goal = plan.root_goal.clone();
    let workspace = parallel_make_workspace(plan)?;
    warn_redundant_container_flag(args);
    let spec = apply_run_execution_overrides(
        &workspace.spec,
        std::slice::from_ref(&workspace.root),
        override_args,
    )?;
    if implicit_local_host {
        print_implicit_local_host_notice(&root_goal);
    }
    let observer =
        ParallelMakeOutputObserver::new(&workspace.goals, parallel_output_override(args));
    let exit_code = submit(
        &spec,
        std::slice::from_ref(&workspace.root),
        workspace.goals.len(),
        true,
        args,
        Some(&observer),
    )
    .await?;
    let exit_code = observer
        .first_failure(&workspace.goals)?
        .unwrap_or(exit_code);
    Ok(MakeRunOutcome { exit_code })
}

async fn submit(
    spec: &tak_core::model::WorkspaceSpec,
    targets: &[tak_core::model::TaskLabel],
    max_parallel_jobs: usize,
    keep_going: bool,
    args: &MakeArgs,
    renderer: Option<&dyn crate::cli::daemon_run::PersistedEventRenderer>,
) -> Result<i32> {
    let workspace = crate::cli::daemon_run::build_workspace(&spec.root)?;
    let submission = resolved::submission(
        spec,
        targets,
        max_parallel_jobs,
        keep_going,
        &args.pass_env,
        workspace.descriptor.clone(),
    )
    .await?;
    let status = match renderer {
        Some(renderer) => {
            crate::cli::daemon_run::submit_resolved_exit_code_with_renderer(
                &spec.root, submission, workspace, renderer,
            )
            .await
        }
        None => {
            crate::cli::daemon_run::submit_resolved_exit_code(&spec.root, submission, workspace)
                .await
        }
    }
    .context("tak make execution requires local takd")?;
    Ok((0..=u8::MAX)
        .find(|code| status == std::process::ExitCode::from(*code))
        .map_or(1, i32::from))
}

fn print_implicit_local_host_notice(goal: &str) {
    eprintln!(
        "info: no Tak execution configuration found for Make goal `{goal}`; running locally outside \
         a container. To run remotely, set `# tak: default.execution=remote` plus a default \
         container image or Dockerfile, add equivalent annotations to this goal, or pass \
         `--remote` with a container source."
    );
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

fn parallel_output_override(args: &MakeArgs) -> Option<ParallelOutputMode> {
    args.parallel_output.map(|mode| match mode {
        MakeParallelOutputArg::Live => ParallelOutputMode::Live,
        MakeParallelOutputArg::Grouped => ParallelOutputMode::Grouped,
    })
}
