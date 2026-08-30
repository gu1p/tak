use anyhow::{Result, bail};
use tak_core::v2::{
    Affinity, AuthoredDefaults, AuthoredModule, AuthoredTask, Execution, LocalExecution, PassEnv,
    RemoteExecution, RemoteSelection, Session, SessionReuse,
};

use super::v2_wire as wire;
use super::v2_wire_primitives::{convert_output, convert_step};

pub(super) fn into_domain(module: wire::Module) -> Result<AuthoredModule> {
    if module.kind != "module_spec_v2" || module.spec_version != 2 {
        bail!("expected the selected module_spec(spec_version=2) value");
    }
    if !module.limiters.is_empty() || !module.queues.is_empty() {
        bail!("v2 limiter and queue decoding is not active in this build");
    }
    let defaults = convert_defaults(module.defaults)?;
    let tasks = module
        .tasks
        .into_iter()
        .map(convert_task)
        .collect::<Result<Vec<_>>>()?;
    for task in &tasks {
        validate_task_affinity(
            task.execution.as_ref().or(defaults.execution.as_ref()),
            task.affinity.as_ref(),
        )?;
    }
    Ok(AuthoredModule {
        project_id: module.project_id,
        tasks,
        includes: module.includes.into_iter().map(convert_output).collect(),
        exclude: module.exclude,
        defaults,
    })
}

fn convert_defaults(defaults: wire::Defaults) -> Result<AuthoredDefaults> {
    if defaults.kind != "defaults_v2" {
        bail!("module_spec(defaults=...) must be produced by Defaults(...)");
    }
    if defaults.queue.is_some() || defaults.retry.is_some() || defaults.container.is_some() {
        bail!("v2 queue, retry, and container defaults are not active in this build");
    }
    Ok(AuthoredDefaults {
        execution: defaults.execution.map(convert_execution).transpose()?,
        pass_env: PassEnv::new(defaults.pass_env)?,
        tags: defaults.tags,
    })
}

fn convert_task(task: wire::Task) -> Result<AuthoredTask> {
    if !task.needs.is_empty()
        || task.queue.is_some()
        || task.retry.is_some()
        || task.context.is_some()
        || task.timeout_s.is_some()
        || task.session.is_some()
        || task.cascade_session
    {
        bail!("this v2 task uses fields not active in this build");
    }
    let execution = task.execution.map(convert_execution).transpose()?;
    let affinity = task.affinity.map(convert_affinity).transpose()?;
    validate_task_affinity(execution.as_ref(), affinity.as_ref())?;
    Ok(AuthoredTask {
        name: task.name,
        doc: task.doc,
        deps: task.deps,
        steps: task.steps.into_iter().map(convert_step).collect(),
        outputs: task.outputs.into_iter().map(convert_output).collect(),
        execution,
        idempotent: task.idempotent,
        pass_env: PassEnv::new(task.pass_env)?,
        affinity,
        tags: task.tags,
    })
}

fn validate_task_affinity(
    execution: Option<&Execution>,
    task_affinity: Option<&Affinity>,
) -> Result<()> {
    let session = match execution {
        Some(Execution::LocalOnly { local }) => local.session.as_deref(),
        Some(Execution::RemoteOnly { remote }) => remote.session.as_deref(),
        None => None,
    };
    if let Some(session) = session {
        session.effective_affinity(task_affinity)?;
    }
    Ok(())
}

fn convert_execution(execution: wire::Execution) -> Result<Execution> {
    match execution {
        wire::Execution::LocalOnly { local } => {
            if local.container.is_some() {
                bail!("v2 local containers are not active in this build");
            }
            Ok(Execution::LocalOnly {
                local: LocalExecution {
                    session: local
                        .session
                        .map(|session| convert_session(*session))
                        .transpose()?
                        .map(Box::new),
                },
            })
        }
        wire::Execution::RemoteOnly { remote } => {
            if remote.container.is_some() {
                bail!("v2 remote containers are not active in this build");
            }
            Ok(Execution::RemoteOnly {
                remote: RemoteExecution {
                    pool: remote.pool,
                    required_tags: remote.required_tags,
                    required_capabilities: remote.required_capabilities,
                    transport: remote.transport.map(convert_transport),
                    selection: convert_selection(remote.selection),
                    session: remote
                        .session
                        .map(|session| convert_session(*session))
                        .transpose()?
                        .map(Box::new),
                },
            })
        }
    }
}

fn convert_session(session: wire::Session) -> Result<Session> {
    if session.kind != "session_v2" || session.context.is_some() {
        bail!("invalid v2 session payload");
    }
    let reuse = convert_reuse(session.reuse)?;
    let affinity = session.affinity.map(convert_affinity).transpose()?;
    let display_name = session.name.clone().unwrap_or_else(|| session.id.clone());
    let mut result = Session::new(display_name, reuse, affinity)?;
    result.id = session.id;
    result.name = session.name;
    result.execution = session
        .execution
        .map(|execution| convert_execution(*execution))
        .transpose()?
        .map(Box::new);
    Ok(result)
}

fn convert_reuse(reuse: wire::Reuse) -> Result<SessionReuse> {
    match reuse {
        wire::Reuse::Workspace => Ok(SessionReuse::Workspace),
        wire::Reuse::Paths { paths } => Ok(SessionReuse::Paths {
            paths: paths.into_iter().map(convert_output).collect(),
        }),
        wire::Reuse::SharedWorkspace { max_parallel_tasks } => {
            Ok(SessionReuse::shared_workspace(max_parallel_tasks)?)
        }
        wire::Reuse::Container => Ok(SessionReuse::Container),
    }
}

fn convert_affinity(affinity: wire::Affinity) -> Result<Affinity> {
    match affinity {
        wire::Affinity::PreferSameNode { group } => Ok(Affinity::prefer_same_node(group)?),
        wire::Affinity::RequireSameNode { group } => Ok(Affinity::require_same_node(group)?),
    }
}

fn convert_selection(selection: wire::Selection) -> RemoteSelection {
    match selection {
        wire::Selection::Balanced => RemoteSelection::Balanced,
        wire::Selection::Sequential => RemoteSelection::Sequential,
        wire::Selection::RoundRobin => RemoteSelection::RoundRobin,
    }
}

fn convert_transport(transport: wire::Transport) -> String {
    match transport {
        wire::Transport::Direct => "direct",
        wire::Transport::Any => "any",
        wire::Transport::Tor => "tor",
    }
    .to_owned()
}
