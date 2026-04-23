use std::collections::{BTreeMap, HashMap};
use std::process::ExitCode;
use std::sync::Arc;

use tak_core::model::{
    CurrentStateSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_resolved_task};

use super::run_output::StdStreamOutputObserver;
use super::run_overrides::{
    RunExecutionOverrideArgs, apply_run_execution_overrides, warn_redundant_remote_container_flag,
};
use super::*;

pub(super) struct ExecCliArgs {
    pub(super) cwd: Option<String>,
    pub(super) env: Vec<String>,
    pub(super) local: bool,
    pub(super) remote: bool,
    pub(super) container: bool,
    pub(super) container_image: Option<String>,
    pub(super) container_dockerfile: Option<String>,
    pub(super) container_build_context: Option<String>,
    pub(super) argv: Vec<String>,
}

pub(super) async fn run_exec_command(args: ExecCliArgs) -> Result<ExitCode> {
    let workspace_root = std::env::current_dir().context("failed to resolve current directory")?;
    let task = synthetic_exec_task(args.argv, args.cwd, parse_exec_env(&args.env)?);
    let label = task.label.clone();
    let mut tasks = std::collections::BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "tak-exec".to_string(),
        root: workspace_root,
        tasks,
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };
    if warn_redundant_remote_container_flag(args.remote, args.container) {
        eprintln!(
            "warning: --container is redundant with --remote; remote execution already implies a containerized runtime"
        );
    }
    let spec = apply_run_execution_overrides(
        &spec,
        std::slice::from_ref(&label),
        RunExecutionOverrideArgs {
            local: args.local,
            remote: args.remote,
            container: args.container,
            container_image: args.container_image.as_deref(),
            container_dockerfile: args.container_dockerfile.as_deref(),
            container_build_context: args.container_build_context.as_deref(),
        },
    )?;
    let task = spec
        .tasks
        .get(&label)
        .ok_or_else(|| anyhow!("missing synthetic exec task"))?;

    let result = run_resolved_task(
        task,
        &spec.root,
        &RunOptions {
            output_observer: Some(Arc::new(StdStreamOutputObserver::default())),
            ..RunOptions::default()
        },
    )
    .await?;

    if result.success {
        return Ok(ExitCode::SUCCESS);
    }

    Ok(exit_code_from_task_result(result.exit_code))
}

fn synthetic_exec_task(
    argv: Vec<String>,
    cwd: Option<String>,
    env: BTreeMap<String, String>,
) -> ResolvedTask {
    ResolvedTask {
        label: TaskLabel {
            package: "//".to_string(),
            name: "exec".to_string(),
        },
        doc: "Synthetic command created by `tak exec`.".to_string(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd { argv, cwd, env }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: TaskExecutionSpec::default(),
        tags: vec!["exec".to_string()],
    }
}

fn parse_exec_env(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for entry in entries {
        let Some((key, value)) = entry.split_once('=') else {
            bail!("invalid --env value `{entry}`; expected KEY=VALUE");
        };
        if key.is_empty() {
            bail!("invalid --env value `{entry}`; key cannot be empty");
        }
        env.insert(key.to_string(), value.to_string());
    }
    Ok(env)
}

fn exit_code_from_task_result(code: Option<i32>) -> ExitCode {
    let normalized = code.unwrap_or(1).clamp(1, u8::MAX as i32) as u8;
    ExitCode::from(normalized)
}
