use std::num::NonZeroU32;

use anyhow::{Result, anyhow, bail};
use tak_core::model::{BackoffDef, OutputSelectorSpec, ResolvedTask, StepDef};
use tak_core::v2::{
    OutputSelector, ResolvedTaskUnit, ResourceRequest, RetryJitter, RetryPolicy, RuntimeResources,
    Step,
};

pub(super) fn unit(
    task: &ResolvedTask,
    job_id: String,
    pass_env_names: &[String],
) -> Result<ResolvedTaskUnit> {
    Ok(ResolvedTaskUnit {
        task_id: super::canonical(&task.label),
        job_id,
        dependencies: task.deps.iter().map(super::canonical).collect(),
        steps: task.steps.iter().map(step).collect(),
        outputs: task.outputs.iter().map(output).collect::<Result<_>>()?,
        pass_env_names: pass_env_names.to_vec(),
        idempotent: false,
        affinity: None,
        timeout_s: task.timeout_s,
        runtime: super::runtime::from_execution(&task.execution)?,
    })
}

pub(super) fn resources(task: &ResolvedTask) -> Result<ResourceRequest> {
    let limits = super::runtime::from_execution(&task.execution)?
        .as_ref()
        .and_then(tak_core::v2::TaskRuntime::resources);
    Ok(limits.map_or_else(ResourceRequest::default, resource_request))
}

pub(super) fn retry(task: &ResolvedTask) -> Result<RetryPolicy> {
    let max_attempts = NonZeroU32::new(task.retry.attempts)
        .ok_or_else(|| anyhow!("Make retry attempts must be positive"))?;
    let (backoff_millis, max_backoff_millis, jitter) = match task.retry.backoff {
        BackoffDef::Fixed { seconds } => {
            let millis = duration_millis(seconds)?;
            (millis, millis, RetryJitter::None)
        }
        BackoffDef::ExpJitter { min_s, max_s, .. } => (
            duration_millis(min_s)?,
            duration_millis(max_s)?,
            RetryJitter::Full,
        ),
    };
    Ok(RetryPolicy {
        max_attempts,
        on_exit: task.retry.on_exit.clone(),
        backoff_millis,
        max_backoff_millis,
        jitter,
    })
}

fn step(value: &StepDef) -> Step {
    match value {
        StepDef::Cmd { argv, cwd, env } => Step::Cmd {
            argv: argv.clone(),
            cwd: cwd.clone(),
            env: env.clone(),
        },
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => Step::Script {
            path: path.clone(),
            argv: argv.clone(),
            interpreter: interpreter.clone(),
            cwd: cwd.clone(),
            env: env.clone(),
        },
    }
}

fn output(value: &OutputSelectorSpec) -> Result<OutputSelector> {
    match value {
        OutputSelectorSpec::Path(path) if path.anchor == tak_core::model::PathAnchor::Workspace => {
            Ok(OutputSelector::Path {
                value: path.path.clone(),
            })
        }
        OutputSelectorSpec::Glob { pattern } => Ok(OutputSelector::Glob {
            value: pattern.clone(),
        }),
        OutputSelectorSpec::Path(_) => bail!("Make output paths must be workspace-relative"),
    }
}

fn resource_request(value: RuntimeResources) -> ResourceRequest {
    ResourceRequest {
        cpu_millis: value.cpu_millis,
        memory_bytes: value.memory_bytes,
        execution_slots: NonZeroU32::MIN,
    }
}

fn duration_millis(seconds: f64) -> Result<u64> {
    let millis = (seconds * 1_000.0).round();
    if !seconds.is_finite() || seconds < 0.0 || millis > u64::MAX as f64 {
        bail!("Make retry backoff is invalid")
    }
    Ok(millis as u64)
}
