use anyhow::{Result, bail};
use tak_core::v2::{AuthoredDefaults, AuthoredModule, AuthoredTask, PassEnv};

use super::v2_wire as wire;
use super::v2_wire_primitives::{convert_output, convert_step};

mod execution;
mod scheduling;

use execution::{convert_affinity, convert_execution, convert_session, validate_task_affinity};
use scheduling::{claim, limiter, queue, queue_use, retry};

#[cfg(test)]
mod scheduling_tests;

#[cfg(test)]
use scheduling::{duration_millis, scaled_positive_millis, scope};

pub(super) fn into_domain(module: wire::Module) -> Result<AuthoredModule> {
    if module.kind != "module_spec_v2" || module.spec_version != 2 {
        bail!("expected the selected module_spec(spec_version=2) value");
    }
    let limiter_definitions = module
        .limiters
        .into_iter()
        .map(limiter)
        .collect::<Result<Vec<_>>>()?;
    let queue_definitions = module
        .queues
        .into_iter()
        .map(queue)
        .collect::<Result<Vec<_>>>()?;
    let defaults = convert_defaults(module.defaults)?;
    let tasks = module
        .tasks
        .into_iter()
        .map(convert_task)
        .collect::<Result<Vec<_>>>()?;
    for task in &tasks {
        validate_task_affinity(
            task.execution.as_ref().or(defaults.execution.as_ref()),
            task.session.as_ref(),
            task.affinity.as_ref(),
        )?;
    }
    Ok(AuthoredModule {
        project_id: module.project_id,
        tasks,
        limiter_definitions,
        queue_definitions,
        includes: module.includes.into_iter().map(convert_output).collect(),
        exclude: module.exclude,
        defaults,
    })
}

fn convert_defaults(defaults: wire::Defaults) -> Result<AuthoredDefaults> {
    if defaults.kind != "defaults_v2" {
        bail!("module_spec(defaults=...) must be produced by Defaults(...)");
    }
    if defaults.container.is_some() {
        bail!("v2 container defaults are not active in this build");
    }
    Ok(AuthoredDefaults {
        execution: defaults.execution.map(convert_execution).transpose()?,
        retry: defaults.retry.map(retry).transpose()?,
        queue: defaults.queue.map(queue_use).transpose()?,
        pass_env: PassEnv::new(defaults.pass_env)?,
        tags: defaults.tags,
    })
}

fn convert_task(task: wire::Task) -> Result<AuthoredTask> {
    if task.context.is_some() || task.timeout_s.is_some() || task.cascade_session {
        bail!("this v2 task uses fields not active in this build");
    }
    let execution = task.execution.map(convert_execution).transpose()?;
    let session = task
        .session
        .map(|session| convert_session(*session))
        .transpose()?;
    if execution.is_some() && session.is_some() {
        bail!("a v2 task cannot use both execution and use_session")
    }
    if session
        .as_ref()
        .is_some_and(|session| session.execution.is_none())
    {
        bail!("task(use_session=...) requires a session with execution")
    }
    let affinity = task.affinity.map(convert_affinity).transpose()?;
    validate_task_affinity(execution.as_ref(), session.as_ref(), affinity.as_ref())?;
    Ok(AuthoredTask {
        name: task.name,
        doc: task.doc,
        deps: task.deps,
        steps: task.steps.into_iter().map(convert_step).collect(),
        outputs: task.outputs.into_iter().map(convert_output).collect(),
        execution,
        retry: task.retry.map(retry).transpose()?,
        queue: task.queue.map(queue_use).transpose()?,
        limiter_claims: task
            .needs
            .into_iter()
            .map(claim)
            .collect::<Result<Vec<_>>>()?,
        session,
        cascade_session: task.cascade_session,
        idempotent: task.idempotent,
        pass_env: PassEnv::new(task.pass_env)?,
        affinity,
        tags: task.tags,
    })
}
