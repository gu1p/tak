use anyhow::{Result, bail};
use tak_core::v2::{
    AuthoredLimiterClaim, AuthoredLimiterDefinition, AuthoredModule, AuthoredQueueDefinition,
    AuthoredQueueUse, AuthoredTask, HoldMode, LimiterClaim, LimiterDefinition, QueueDefinition,
    QueueDiscipline, RetryPolicy,
};

pub(super) fn retry(module: &AuthoredModule, task: &AuthoredTask) -> RetryPolicy {
    task.retry
        .clone()
        .or_else(|| module.defaults.retry.clone())
        .unwrap_or_default()
}

pub(super) fn queue_name(module: &AuthoredModule, task: &AuthoredTask) -> Result<Option<String>> {
    let Some(queue) = queue_use(module, task) else {
        return Ok(None);
    };
    if queue.slots.get() != 1 || queue.priority != 0 {
        bail!("v2 queue uses currently require slots=1 and priority=0")
    }
    Ok(Some(queue.name.clone()))
}

pub(super) fn claims(task: &AuthoredTask) -> Vec<LimiterClaim> {
    task.limiter_claims
        .iter()
        .map(|claim| LimiterClaim {
            name: claim.name.clone(),
            amount_millis: claim.amount_millis,
        })
        .collect()
}

pub(super) fn queues(
    module: &AuthoredModule,
    tasks: &[&AuthoredTask],
    worktree_scope_key: &str,
) -> Result<Vec<QueueDefinition>> {
    let mut uses = Vec::<&AuthoredQueueUse>::new();
    for task in tasks {
        if let Some(queue) = queue_use(module, task)
            && !uses
                .iter()
                .any(|item| item.name == queue.name && item.scope == queue.scope)
        {
            uses.push(queue);
        }
    }
    let mut result = Vec::new();
    for queue_use in uses {
        let definition = unique_queue(module, queue_use)?;
        if definition.discipline != QueueDiscipline::Fifo {
            bail!("v2 priority queues are not active in this build")
        }
        if result
            .iter()
            .any(|existing: &QueueDefinition| existing.name == definition.name)
        {
            bail!("v2 resolved queues require unique names across scopes")
        }
        result.push(QueueDefinition {
            name: definition.name.clone(),
            scope: definition.scope.clone(),
            scope_key: resolved_scope_key(&definition.scope, worktree_scope_key),
            max_parallel_tasks: definition.max_parallel_tasks,
            discipline: definition.discipline,
        });
    }
    Ok(result)
}

pub(super) fn limiters(
    module: &AuthoredModule,
    tasks: &[&AuthoredTask],
    worktree_scope_key: &str,
) -> Result<Vec<LimiterDefinition>> {
    let mut uses = Vec::<&AuthoredLimiterClaim>::new();
    for task in tasks {
        for claim in &task.limiter_claims {
            if let Some(existing) = uses
                .iter()
                .find(|item| item.name == claim.name && item.scope == claim.scope)
            {
                if existing.hold != claim.hold {
                    bail!("v2 limiter `{}` cannot mix hold modes", claim.name)
                }
            } else {
                uses.push(claim);
            }
        }
    }
    let mut result = Vec::new();
    for (index, claim) in uses.iter().enumerate() {
        let definition = unique_limiter(module, claim)?;
        if uses[..index].iter().any(|item| item.name == claim.name) {
            bail!("v2 resolved limiters require unique names across scopes")
        }
        result.push(resolve_limiter(definition, claim.hold, worktree_scope_key)?);
    }
    Ok(result)
}

fn queue_use<'a>(
    module: &'a AuthoredModule,
    task: &'a AuthoredTask,
) -> Option<&'a AuthoredQueueUse> {
    task.queue.as_ref().or(module.defaults.queue.as_ref())
}

fn unique_queue<'a>(
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

fn unique_limiter<'a>(
    module: &'a AuthoredModule,
    claim: &AuthoredLimiterClaim,
) -> Result<&'a AuthoredLimiterDefinition> {
    let matches = module
        .limiter_definitions
        .iter()
        .filter(|item| item.name() == claim.name && item.scope() == &claim.scope)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [definition] => Ok(definition),
        [] => bail!("unknown scoped limiter `{}`", claim.name),
        _ => bail!("duplicate scoped limiter `{}`", claim.name),
    }
}

fn resolve_limiter(
    definition: &AuthoredLimiterDefinition,
    hold: HoldMode,
    worktree_scope_key: &str,
) -> Result<LimiterDefinition> {
    Ok(match definition {
        AuthoredLimiterDefinition::Lock { name, scope } => LimiterDefinition::Lock {
            name: name.clone(),
            scope: scope.clone(),
            scope_key: resolved_scope_key(scope, worktree_scope_key),
            hold,
        },
        AuthoredLimiterDefinition::Resource {
            name,
            scope,
            capacity_millis,
        } => LimiterDefinition::Resource {
            name: name.clone(),
            scope: scope.clone(),
            scope_key: resolved_scope_key(scope, worktree_scope_key),
            capacity_millis: *capacity_millis,
            hold,
        },
        AuthoredLimiterDefinition::ProcessCap {
            name,
            scope,
            max_processes,
        } => LimiterDefinition::ProcessCap {
            name: name.clone(),
            scope: scope.clone(),
            scope_key: resolved_scope_key(scope, worktree_scope_key),
            max_processes: *max_processes,
            hold,
        },
        AuthoredLimiterDefinition::RateLimit {
            name,
            scope,
            permits,
            per_millis,
        } => {
            if hold != HoldMode::AtStart {
                bail!("v2 rate-limit claims require Hold.AtStart")
            }
            LimiterDefinition::RateLimit {
                name: name.clone(),
                scope: scope.clone(),
                scope_key: resolved_scope_key(scope, worktree_scope_key),
                permits: *permits,
                per_millis: *per_millis,
            }
        }
    })
}

fn resolved_scope_key(scope: &tak_core::v2::DefinitionScope, worktree: &str) -> Option<String> {
    (*scope == tak_core::v2::DefinitionScope::Worktree).then(|| worktree.to_owned())
}
