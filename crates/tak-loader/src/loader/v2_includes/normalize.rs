use std::path::{Component, Path};

use anyhow::{Result, anyhow, bail};
use tak_core::v2::{
    AuthoredDefaults, AuthoredModule, ContainerSource, Execution, OutputSelector, PassEnv, Session,
    SessionReuse, Step, TaskRuntime,
};

pub(super) fn module(
    module: &mut AuthoredModule,
    package: &str,
    preserve_defaults: bool,
) -> Result<()> {
    for task in &mut module.tasks {
        if !preserve_defaults {
            apply_defaults(task, &module.defaults)?;
        }
        normalize_task_paths(task, package)?;
    }
    if !preserve_defaults {
        module.defaults = AuthoredDefaults::default();
    }
    module.includes.clear();
    if !package.is_empty() {
        module.exclude = module
            .exclude
            .iter()
            .map(|value| anchored(package, value))
            .collect::<Result<Vec<_>>>()?;
    }
    Ok(())
}

fn apply_defaults(
    task: &mut tak_core::v2::AuthoredTask,
    defaults: &AuthoredDefaults,
) -> Result<()> {
    task.execution = task.execution.take().or(defaults.execution.clone());
    task.retry = task.retry.take().or(defaults.retry.clone());
    task.queue = task.queue.take().or(defaults.queue.clone());
    task.pass_env = PassEnv::new(
        defaults
            .pass_env
            .as_strs()
            .into_iter()
            .chain(task.pass_env.as_strs()),
    )?;
    let mut tags = defaults.tags.clone();
    tags.append(&mut task.tags);
    tags.sort();
    tags.dedup();
    task.tags = tags;
    Ok(())
}

pub(super) fn package(root: &Path, tasks_file: &Path) -> Result<String> {
    let parent = tasks_file
        .parent()
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", tasks_file.display()))?;
    Ok(parent
        .strip_prefix(root)?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/"))
}

pub(super) fn label(value: &str, package: &str) -> Result<String> {
    if value.starts_with("//") {
        return Ok(value.to_owned());
    }
    let name = value.strip_prefix(':').unwrap_or(value);
    if name.is_empty() || name.contains('/') || name.contains(':') {
        bail!("invalid task label `{value}`")
    }
    Ok(if package.is_empty() {
        format!("//:{name}")
    } else {
        format!("//{package}:{name}")
    })
}

fn normalize_task_paths(task: &mut tak_core::v2::AuthoredTask, package: &str) -> Result<()> {
    for step in &mut task.steps {
        match step {
            Step::Cmd { cwd, .. } => *cwd = Some(anchored(package, cwd.as_deref().unwrap_or("."))?),
            Step::Script { path, cwd, .. } => {
                *path = anchored(package, path)?;
                *cwd = Some(anchored(package, cwd.as_deref().unwrap_or("."))?);
            }
        }
    }
    for output in &mut task.outputs {
        normalize_selector(output, package)?;
    }
    if let Some(execution) = &mut task.execution {
        normalize_execution(execution, package)?;
    }
    if let Some(session) = &mut task.session {
        normalize_session(session, package)?;
    }
    super::normalize_context::paths(task.context.as_mut(), package)?;
    Ok(())
}

fn normalize_execution(execution: &mut Execution, package: &str) -> Result<()> {
    if let Execution::FirstAvailable { placements, .. } = execution {
        for placement in placements {
            normalize_execution(placement, package)?;
        }
        return Ok(());
    }
    let (runtime, session) = match execution {
        Execution::LocalOnly { local } => (&mut local.runtime, &mut local.session),
        Execution::RemoteOnly { remote } => (&mut remote.runtime, &mut remote.session),
        Execution::FirstAvailable { .. } => unreachable!(),
    };
    if let Some(runtime) = runtime {
        normalize_runtime(runtime, package)?;
    }
    if let Some(session) = session {
        normalize_session(session, package)?;
    }
    Ok(())
}

fn normalize_runtime(runtime: &mut TaskRuntime, package: &str) -> Result<()> {
    let TaskRuntime::Container { source, mounts, .. } = runtime;
    if let ContainerSource::Dockerfile {
        dockerfile,
        build_context,
    } = source
    {
        *dockerfile = anchored(package, dockerfile)?;
        *build_context = anchored(package, build_context)?;
    }
    for mount in &mut *mounts {
        mount.source = anchored(package, &mount.source)?;
    }
    mounts.sort();
    mounts.dedup();
    Ok(())
}

fn normalize_session(session: &mut Session, package: &str) -> Result<()> {
    session.id = format!("{package}:{}", session.id);
    super::normalize_context::paths(session.context.as_mut(), package)?;
    if let SessionReuse::Paths { paths } = &mut session.reuse {
        for selector in paths {
            normalize_selector(selector, package)?;
        }
    }
    if let Some(execution) = &mut session.execution {
        normalize_execution(execution, package)?;
    }
    Ok(())
}

fn normalize_selector(selector: &mut OutputSelector, package: &str) -> Result<()> {
    let value = match selector {
        OutputSelector::Path { value } | OutputSelector::Glob { value } => value,
    };
    *value = anchored(package, value)?;
    Ok(())
}

pub(super) fn anchored(package: &str, value: &str) -> Result<String> {
    let (rooted, value) = value
        .strip_prefix("//")
        .map_or((false, value), |value| (true, value));
    if rooted && value.is_empty() {
        return Ok(".".into());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("package-relative path `{value}` escapes its TASKS.py directory")
    }
    if rooted || package.is_empty() {
        return Ok(value.trim_end_matches('/').to_owned());
    }
    let joined = if value == "." {
        package.into()
    } else {
        format!("{package}/{value}")
    };
    Ok(joined.trim_end_matches('/').to_owned())
}
