use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Transaction;
use tak_core::v2::{RemoteSelection, ResolvedJob};

use crate::daemon::scheduler::SchedulerNode;

use super::constraints::{self, Context};
use accounting::reserved_usage;
use live_inventory::matches_live_requirements;
use scoring::{Usage, has_capacity, score_node};

mod accounting;
#[cfg(test)]
mod accounting_tests;
mod live_inventory;
#[cfg(test)]
mod live_inventory_tests;
mod locality;
mod scoring;
#[cfg(test)]
mod tests;

pub(super) struct AffinitySelection<'a> {
    pub(super) eligible_nodes: Option<&'a BTreeSet<String>>,
    pub(super) preferred_node: Option<&'a str>,
    pub(super) lost_nodes: &'a BTreeSet<String>,
}

pub(super) fn select_node<'a>(
    transaction: &Transaction<'_>,
    nodes: &'a [SchedulerNode],
    job: &ResolvedJob,
    cursor: u64,
    workspace_fingerprint: &str,
    affinity: &AffinitySelection<'_>,
    constraint_context: &Context<'_>,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let nodes_by_id = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let tiers = job
        .placement_candidates
        .iter()
        .map(|candidate| candidate.tier)
        .collect::<BTreeSet<_>>();
    for (tier_index, tier) in tiers.into_iter().enumerate() {
        let selected = match job.placement_policy.selection {
            RemoteSelection::Balanced => select_balanced(
                transaction,
                &nodes_by_id,
                job,
                tier,
                workspace_fingerprint,
                affinity,
                constraint_context,
            )?,
            RemoteSelection::RoundRobin | RemoteSelection::Sequential => select_ordered(
                transaction,
                &nodes_by_id,
                job,
                tier,
                (tier_index == 0).then_some(cursor),
                affinity,
                constraint_context,
            )?,
        };
        if selected.is_some() {
            return Ok(selected);
        }
    }
    Ok(None)
}

fn select_ordered<'a>(
    transaction: &Transaction<'_>,
    nodes: &BTreeMap<&str, &'a SchedulerNode>,
    job: &ResolvedJob,
    tier: u32,
    primary_cursor: Option<u64>,
    affinity: &AffinitySelection<'_>,
    constraint_context: &Context<'_>,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let candidates = job
        .placement_candidates
        .iter()
        .filter(|candidate| candidate.tier == tier)
        .collect::<Vec<_>>();
    let count = candidates.len();
    let start = match job.placement_policy.selection {
        RemoteSelection::RoundRobin => usize::try_from(primary_cursor.unwrap_or(0) % count as u64)?,
        RemoteSelection::Sequential => 0,
        RemoteSelection::Balanced => unreachable!("balanced uses scored selection"),
    };
    for offset in 0..count {
        let index = (start + offset) % count;
        let candidate = candidates[index];
        if !candidate_is_eligible(candidate.node_id.as_str(), affinity) {
            continue;
        }
        let Some(node) = nodes.get(candidate.node_id.as_str()) else {
            continue;
        };
        if candidate.transport != node.transport || !matches_live_requirements(candidate, node) {
            continue;
        }
        let usage = reserved_usage(transaction, node)?;
        if has_capacity(node, usage, job.resources)
            && constraints::can_acquire(transaction, constraint_context, job, node)?
        {
            let next = (primary_cursor.is_some()
                && job.placement_policy.selection == RemoteSelection::RoundRobin)
                .then_some(((index + 1) % count) as u64);
            return Ok(Some((node, next)));
        }
    }
    Ok(None)
}

fn select_balanced<'a>(
    transaction: &Transaction<'_>,
    nodes: &BTreeMap<&str, &'a SchedulerNode>,
    job: &ResolvedJob,
    tier: u32,
    workspace_fingerprint: &str,
    affinity: &AffinitySelection<'_>,
    constraint_context: &Context<'_>,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let mut best = None;
    for (index, candidate) in job
        .placement_candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.tier == tier)
    {
        if !candidate_is_eligible(candidate.node_id.as_str(), affinity) {
            continue;
        }
        let Some(node) = nodes.get(candidate.node_id.as_str()).copied() else {
            continue;
        };
        if candidate.transport != node.transport || !matches_live_requirements(candidate, node) {
            continue;
        }
        let usage = reserved_usage(transaction, node)?;
        if !has_capacity(node, usage, job.resources) {
            continue;
        }
        if !constraints::can_acquire(transaction, constraint_context, job, node)? {
            continue;
        }
        let local = locality::present(
            node,
            job,
            workspace_fingerprint,
            affinity.preferred_node == Some(node.node_id.as_str()),
            constraint_context,
        )?;
        let score = score_node(node, usage, job.resources, local);
        if best
            .as_ref()
            .is_none_or(|(_, best_score, best_index)| (score, index) < (*best_score, *best_index))
        {
            best = Some((node, score, index));
        }
    }
    Ok(best.map(|(node, _, _)| (node, None)))
}

fn candidate_is_eligible(node_id: &str, affinity: &AffinitySelection<'_>) -> bool {
    !affinity.lost_nodes.contains(node_id)
        && affinity
            .eligible_nodes
            .is_none_or(|eligible| eligible.contains(node_id))
}
