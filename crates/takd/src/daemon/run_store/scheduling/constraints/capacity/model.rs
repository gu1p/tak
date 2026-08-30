use anyhow::Result;
use tak_core::v2::{DefinitionScope, ResolvedJob, ResolvedRun};

use super::limiter::{limiter_name, properties};

pub(in crate::daemon::run_store::scheduling) struct Context<'a> {
    pub(in crate::daemon::run_store::scheduling) run_id: &'a str,
    pub(in crate::daemon::run_store::scheduling) job_id: &'a str,
    pub(in crate::daemon::run_store::scheduling) submitter_id: &'a str,
    pub(in crate::daemon::run_store::scheduling) run: &'a ResolvedRun,
    pub(in crate::daemon::run_store::scheduling) now_ms: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Key {
    namespace: &'static str,
    name: String,
    owner: Owner,
}

impl Key {
    pub(super) fn is_node_scoped(&self) -> bool {
        self.owner.scope == DefinitionScope::Node
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Owner {
    scope: DefinitionScope,
    identity: String,
    scope_key: Option<String>,
}

#[derive(Clone)]
pub(super) struct Constraint {
    pub(super) key: Key,
    pub(super) amount: u64,
    pub(super) capacity: u64,
    lease: Lease,
    pub(super) reserved_at_ms: u64,
    pub(super) accepted: bool,
    pub(super) released: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Lease {
    During,
    AtStart,
    Rate(u64),
}

impl Constraint {
    pub(super) fn is_queue(&self) -> bool {
        self.key.namespace == "queue"
    }

    pub(super) fn active(&self, now_ms: u64) -> bool {
        match self.lease {
            Lease::During => !self.released,
            Lease::AtStart => !self.accepted && !self.released,
            Lease::Rate(window) => self.reserved_at_ms.saturating_add(window) > now_ms,
        }
    }
}

pub(super) fn constraints(
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<Vec<Constraint>> {
    let mut result = Vec::new();
    if let Some(name) = &job.queue {
        let definition = context
            .run
            .queue_definitions
            .iter()
            .find(|definition| &definition.name == name)
            .ok_or_else(|| anyhow::anyhow!("unknown queue `{name}`"))?;
        result.push(constraint(
            "queue",
            name,
            owner(
                context,
                &definition.scope,
                definition.scope_key.as_deref(),
                node_id,
            )?,
            1,
            u64::from(definition.max_parallel_tasks.get()),
            Lease::During,
        ));
    }
    for claim in &job.limiter_claims {
        let definition = context
            .run
            .limiter_definitions
            .iter()
            .find(|definition| limiter_name(definition) == claim.name)
            .ok_or_else(|| anyhow::anyhow!("unknown limiter `{}`", claim.name))?;
        let (scope, scope_key, capacity, lease) = properties(definition);
        result.push(constraint(
            "limiter",
            &claim.name,
            owner(context, scope, scope_key, node_id)?,
            claim.amount_millis.get(),
            capacity,
            lease,
        ));
    }
    Ok(result)
}

fn constraint(
    namespace: &'static str,
    name: &str,
    owner: Owner,
    amount: u64,
    capacity: u64,
    lease: Lease,
) -> Constraint {
    Constraint {
        key: Key {
            namespace,
            name: name.into(),
            owner,
        },
        amount,
        capacity,
        lease,
        reserved_at_ms: 0,
        accepted: false,
        released: false,
    }
}

fn owner(
    context: &Context<'_>,
    scope: &DefinitionScope,
    scope_key: Option<&str>,
    node_id: &str,
) -> Result<Owner> {
    let identity = match scope {
        DefinitionScope::Run => context.run_id,
        DefinitionScope::Submitter => context.submitter_id,
        DefinitionScope::Project => context.run.project_id.as_str(),
        DefinitionScope::Worktree => scope_key
            .ok_or_else(|| anyhow::anyhow!("worktree scheduling scope is missing its owner key"))?,
        DefinitionScope::Node => node_id,
    };
    Ok(Owner {
        scope: scope.clone(),
        identity: identity.into(),
        scope_key: scope_key.map(str::to_owned),
    })
}
