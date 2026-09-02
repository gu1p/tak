use std::num::NonZeroU32;

use anyhow::{Result, bail};
use tak_core::v2::{
    AuthoredModule, AuthoredQueueDefinition, AuthoredQueueUse, AuthoredTask, QueueDefinition,
};

pub(in crate::cli::daemon_run::resolve) struct ResolvedUse {
    pub(in crate::cli::daemon_run::resolve) name: Option<String>,
    pub(in crate::cli::daemon_run::resolve) slots: NonZeroU32,
    pub(in crate::cli::daemon_run::resolve) priority: i32,
}

pub(in crate::cli::daemon_run::resolve) fn resolved_use(
    module: &AuthoredModule,
    task: &AuthoredTask,
) -> Result<ResolvedUse> {
    let Some(queue) = authored_use(module, task) else {
        return Ok(ResolvedUse {
            name: None,
            slots: NonZeroU32::MIN,
            priority: 0,
        });
    };
    unique(module, queue)?;
    Ok(ResolvedUse {
        name: Some(queue.name.clone()),
        slots: queue.slots,
        priority: queue.priority,
    })
}

pub(in crate::cli::daemon_run::resolve) fn definitions(
    module: &AuthoredModule,
    tasks: &[&AuthoredTask],
    worktree_scope_key: &str,
) -> Result<Vec<QueueDefinition>> {
    let mut uses = Vec::<&AuthoredQueueUse>::new();
    for task in tasks {
        if let Some(queue) = authored_use(module, task)
            && !uses
                .iter()
                .any(|item| item.name == queue.name && item.scope == queue.scope)
        {
            uses.push(queue);
        }
    }
    let mut result = Vec::new();
    for queue_use in uses {
        let definition = unique(module, queue_use)?;
        if result
            .iter()
            .any(|existing: &QueueDefinition| existing.name == definition.name)
        {
            bail!("v2 resolved queues require unique names across scopes")
        }
        result.push(QueueDefinition {
            name: definition.name.clone(),
            scope: definition.scope.clone(),
            scope_key: super::resolved_scope_key(&definition.scope, worktree_scope_key),
            max_parallel_tasks: definition.max_parallel_tasks,
            discipline: definition.discipline,
        });
    }
    Ok(result)
}

fn authored_use<'a>(
    module: &'a AuthoredModule,
    task: &'a AuthoredTask,
) -> Option<&'a AuthoredQueueUse> {
    task.queue.as_ref().or(module.defaults.queue.as_ref())
}

fn unique<'a>(
    module: &'a AuthoredModule,
    reference: &AuthoredQueueUse,
) -> Result<&'a AuthoredQueueDefinition> {
    let matches = module
        .queue_definitions
        .iter()
        .filter(|item| item.name == reference.name && item.scope == reference.scope)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [definition] => Ok(definition),
        [] => bail!("unknown scoped queue `{}`", reference.name),
        _ => bail!("duplicate scoped queue `{}`", reference.name),
    }
}
