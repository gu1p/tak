use std::path::{Component, Path};

use crate::v2::{ResolvedTaskUnit, Step};

use super::ResolvedRunError;

pub(super) fn validate(task: &ResolvedTaskUnit) -> Result<(), ResolvedRunError> {
    for step in &task.steps {
        let (script, cwd) = match step {
            Step::Cmd { cwd, .. } => (None, cwd.as_deref()),
            Step::Script { path, cwd, .. } => (Some(path.as_str()), cwd.as_deref()),
        };
        if cwd.is_some_and(|path| !is_safe(path)) {
            return Err(ResolvedRunError::new(format!(
                "task `{}` step working directory must stay inside the workspace",
                task.task_id
            )));
        }
        if script.is_some_and(|path| !is_safe(path)) {
            return Err(ResolvedRunError::new(format!(
                "task `{}` script path must stay inside the workspace",
                task.task_id
            )));
        }
    }
    Ok(())
}

fn is_safe(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let mut depth = 0_usize;
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
