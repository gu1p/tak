use anyhow::Result;
use tak_core::v2::TaskContext;
use tak_loader::V2AuthoredRoot;

use super::graph::{canonical, selected_tasks};
use super::{RunCliArgs, context, sessions};

pub(in crate::cli::daemon_run) fn resolve(
    root: &V2AuthoredRoot,
    args: &RunCliArgs,
) -> Result<Vec<TaskContext>> {
    let selected = selected_tasks(&root.module, &args.labels)?;
    let bindings = sessions::bindings(&root.module, &selected)?;
    let mut contexts = Vec::new();
    for task in selected {
        let task_id = canonical(&task.name)?;
        if let Some(context) =
            context::effective(task.context.as_ref(), bindings[&task_id].session.as_ref())
        {
            contexts.push(context.clone());
        }
    }
    Ok(contexts)
}
