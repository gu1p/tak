use std::path::{Component, Path};

use anyhow::{Result, bail};
use tak_core::v2::TaskContext;

use super::super::v2_wire as wire;

pub(super) fn convert_context(context: wire::Context) -> Result<TaskContext> {
    let mut ignored_paths = Vec::new();
    let mut use_gitignore = false;
    for ignored in context.ignored {
        match ignored {
            wire::Ignore::Path { value } => ignored_paths.push(valid_path(value)?),
            wire::Ignore::Gitignore => use_gitignore = true,
        }
    }
    let mut roots = paths(context.roots)?;
    if roots.is_empty() {
        roots.push(".".into());
    }
    Ok(TaskContext {
        roots,
        ignored_paths,
        use_gitignore,
        include: paths(context.include)?,
    })
}

fn paths(values: Vec<wire::Output>) -> Result<Vec<String>> {
    values
        .into_iter()
        .map(|value| match value {
            wire::Output::Path { value } => valid_path(value),
            wire::Output::Glob { .. } => bail!("CurrentState accepts path(...), not glob(...)"),
        })
        .collect()
}

fn valid_path(value: String) -> Result<String> {
    let checked = value.strip_prefix("//").unwrap_or(&value);
    let path = Path::new(checked);
    if value.is_empty()
        || checked.is_empty()
        || path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("CurrentState paths must stay inside the workspace")
    }
    Ok(value.trim_end_matches('/').to_owned())
}
