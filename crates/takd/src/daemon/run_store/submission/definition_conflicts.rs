use std::collections::BTreeSet;

use anyhow::{Result, bail};
use rusqlite::Transaction;
use tak_core::v2::{DefinitionScope, LimiterDefinition, ResolvedRun, RunSubmission};

pub(super) fn reject_active_conflicts(
    transaction: &Transaction<'_>,
    submission: &RunSubmission,
    submitter_id: &str,
) -> Result<()> {
    let stored = {
        let mut statement = transaction.prepare(
            "SELECT submitter_id, resolved_json FROM runs \
             WHERE state NOT IN ('succeeded', 'failed', 'cancelled') \
             ORDER BY created_at_ms, run_id",
        )?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (existing_submitter, serialized) in stored {
        let existing: ResolvedRun = serde_json::from_str(&serialized)?;
        reject_queue_conflicts(
            &existing,
            &existing_submitter,
            &submission.run,
            submitter_id,
        )?;
        reject_limiter_conflicts(
            &existing,
            &existing_submitter,
            &submission.run,
            submitter_id,
        )?;
    }
    Ok(())
}

fn reject_queue_conflicts(
    existing: &ResolvedRun,
    existing_submitter: &str,
    submitted: &ResolvedRun,
    submitted_submitter: &str,
) -> Result<()> {
    for new in &submitted.queue_definitions {
        let conflict = existing.queue_definitions.iter().find(|old| {
            old.name == new.name
                && old.scope == new.scope
                && old.scope_key == new.scope_key
                && *old != new
                && owners_overlap(
                    &new.scope,
                    existing,
                    existing_submitter,
                    submitted,
                    submitted_submitter,
                    queue_nodes(existing, &new.name),
                    queue_nodes(submitted, &new.name),
                )
        });
        if let Some(old) = conflict {
            bail!(
                "conflicting queue definition: existing={}; submitted={}",
                serde_json::to_string(old)?,
                serde_json::to_string(new)?
            );
        }
    }
    Ok(())
}

fn reject_limiter_conflicts(
    existing: &ResolvedRun,
    existing_submitter: &str,
    submitted: &ResolvedRun,
    submitted_submitter: &str,
) -> Result<()> {
    for new in &submitted.limiter_definitions {
        let conflict = existing.limiter_definitions.iter().find(|old| {
            name(old) == name(new)
                && scope(old) == scope(new)
                && scope_key(old) == scope_key(new)
                && *old != new
                && owners_overlap(
                    scope(new),
                    existing,
                    existing_submitter,
                    submitted,
                    submitted_submitter,
                    limiter_nodes(existing, name(new)),
                    limiter_nodes(submitted, name(new)),
                )
        });
        if let Some(old) = conflict {
            bail!(
                "conflicting limiter definition: existing={}; submitted={}",
                serde_json::to_string(old)?,
                serde_json::to_string(new)?
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn owners_overlap(
    scope: &DefinitionScope,
    existing: &ResolvedRun,
    existing_submitter: &str,
    submitted: &ResolvedRun,
    submitted_submitter: &str,
    existing_nodes: BTreeSet<String>,
    submitted_nodes: BTreeSet<String>,
) -> bool {
    match scope {
        DefinitionScope::Run => false,
        DefinitionScope::Submitter => existing_submitter == submitted_submitter,
        DefinitionScope::Project => existing.project_id == submitted.project_id,
        DefinitionScope::Worktree => true,
        DefinitionScope::Node => !existing_nodes.is_disjoint(&submitted_nodes),
    }
}

fn queue_nodes(run: &ResolvedRun, name: &str) -> BTreeSet<String> {
    run.jobs
        .iter()
        .filter(|job| job.queue.as_deref() == Some(name))
        .flat_map(|job| job.placement_candidates.iter())
        .map(|candidate| candidate.node_id.clone())
        .collect()
}

fn limiter_nodes(run: &ResolvedRun, name: &str) -> BTreeSet<String> {
    run.jobs
        .iter()
        .filter(|job| job.limiter_claims.iter().any(|claim| claim.name == name))
        .flat_map(|job| job.placement_candidates.iter())
        .map(|candidate| candidate.node_id.clone())
        .collect()
}

fn name(definition: &LimiterDefinition) -> &str {
    match definition {
        LimiterDefinition::Lock { name, .. }
        | LimiterDefinition::RateLimit { name, .. }
        | LimiterDefinition::ProcessCap { name, .. }
        | LimiterDefinition::Resource { name, .. } => name,
    }
}

fn scope(definition: &LimiterDefinition) -> &DefinitionScope {
    match definition {
        LimiterDefinition::Lock { scope, .. }
        | LimiterDefinition::RateLimit { scope, .. }
        | LimiterDefinition::ProcessCap { scope, .. }
        | LimiterDefinition::Resource { scope, .. } => scope,
    }
}

fn scope_key(definition: &LimiterDefinition) -> &Option<String> {
    match definition {
        LimiterDefinition::Lock { scope_key, .. }
        | LimiterDefinition::RateLimit { scope_key, .. }
        | LimiterDefinition::ProcessCap { scope_key, .. }
        | LimiterDefinition::Resource { scope_key, .. } => scope_key,
    }
}
