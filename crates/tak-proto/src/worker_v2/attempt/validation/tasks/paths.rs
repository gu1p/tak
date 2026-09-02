use std::path::{Component, Path};

use anyhow::{Result, bail};
use tak_core::v2::{OutputSelector, ResolvedTaskUnit, Step};

pub(super) fn validate(task: &ResolvedTaskUnit) -> Result<()> {
    for step in &task.steps {
        let (script, cwd) = match step {
            Step::Cmd { cwd, .. } => (None, cwd.as_deref()),
            Step::Script { path, cwd, .. } => (Some(path.as_str()), cwd.as_deref()),
        };
        if cwd.is_some_and(|value| !step_path(value))
            || script.is_some_and(|value| !step_path(value))
        {
            bail!("worker task step path must stay inside the workspace");
        }
    }
    for output in &task.outputs {
        let safe = match output {
            OutputSelector::Path { value } => strict_path(value),
            OutputSelector::Glob { value } => safe_glob(value),
        };
        if !safe {
            bail!("worker task output must stay inside the workspace");
        }
    }
    Ok(())
}

fn step_path(value: &str) -> bool {
    if value.is_empty() || value.contains(['\\', '\0']) {
        return false;
    }
    let mut depth = 0;
    for component in Path::new(value).components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => depth -= 1,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn strict_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', '\0'])
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn safe_glob(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', '\0'])
        && !Path::new(value).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}
