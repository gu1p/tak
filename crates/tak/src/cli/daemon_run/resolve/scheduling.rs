use anyhow::{Result, bail};
use tak_core::v2::{
    AuthoredLimiterClaim, AuthoredLimiterDefinition, AuthoredModule, AuthoredTask, HoldMode,
    LimiterClaim, LimiterDefinition, RetryPolicy,
};

mod queue;

pub(super) use queue::{definitions as queues, resolved_use as queue};

pub(super) fn retry(module: &AuthoredModule, task: &AuthoredTask) -> RetryPolicy {
    task.retry
        .clone()
        .or_else(|| module.defaults.retry.clone())
        .unwrap_or_default()
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
            unit,
        } => LimiterDefinition::Resource {
            name: name.clone(),
            scope: scope.clone(),
            scope_key: resolved_scope_key(scope, worktree_scope_key),
            capacity_millis: *capacity_millis,
            unit: unit.clone(),
            hold,
        },
        AuthoredLimiterDefinition::ProcessCap {
            name,
            scope,
            max_processes,
            match_pattern,
        } => LimiterDefinition::ProcessCap {
            name: name.clone(),
            scope: scope.clone(),
            scope_key: resolved_scope_key(scope, worktree_scope_key),
            max_processes: *max_processes,
            match_pattern: match_pattern.clone(),
            hold,
        },
        AuthoredLimiterDefinition::RateLimit {
            name,
            scope,
            burst,
            refill_millis_per_second,
        } => {
            if hold != HoldMode::AtStart {
                bail!("v2 rate-limit claims require Hold.AtStart")
            }
            LimiterDefinition::RateLimit {
                name: name.clone(),
                scope: scope.clone(),
                scope_key: resolved_scope_key(scope, worktree_scope_key),
                burst: *burst,
                refill_millis_per_second: *refill_millis_per_second,
            }
        }
    })
}

fn resolved_scope_key(scope: &tak_core::v2::DefinitionScope, worktree: &str) -> Option<String> {
    (*scope == tak_core::v2::DefinitionScope::Worktree).then(|| worktree.to_owned())
}
