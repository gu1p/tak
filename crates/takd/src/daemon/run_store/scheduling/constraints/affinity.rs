use std::collections::BTreeSet;

use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::{Affinity, ResolvedJob, ResolvedRun};

pub(in crate::daemon::run_store::scheduling) fn eligible_hard_affinity_nodes(
    transaction: &Transaction<'_>,
    run_id: &str,
    run: &ResolvedRun,
    job: &ResolvedJob,
) -> Result<Option<BTreeSet<String>>> {
    let Some(affinity) = &job.affinity else {
        return Ok(None);
    };
    let (group, required) = match affinity {
        Affinity::PreferSameNode { group } => (group, false),
        Affinity::RequireSameNode { group } => (group, true),
    };
    let home = transaction
        .query_row(
            "SELECT node_id FROM run_affinity_bindings \
             WHERE run_id = ?1 AND affinity_group = ?2",
            params![run_id, group],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(home) = home {
        return Ok(required.then(|| BTreeSet::from([home])));
    }
    let mut common = None::<BTreeSet<String>>;
    for member in run.jobs.iter().filter(|member| {
        matches!(&member.affinity, Some(Affinity::RequireSameNode { group: other }) if other == group)
    }) {
        let nodes = member
            .placement_candidates
            .iter()
            .map(|candidate| candidate.node_id.clone())
            .collect::<BTreeSet<_>>();
        common = Some(match common {
            Some(mut common) => {
                common.retain(|node| nodes.contains(node));
                common
            }
            None => nodes,
        });
    }
    Ok(common)
}

pub(in crate::daemon::run_store::scheduling) fn preferred_affinity_home(
    transaction: &Transaction<'_>,
    run_id: &str,
    job: &ResolvedJob,
) -> Result<Option<String>> {
    let Some(Affinity::PreferSameNode { group }) = &job.affinity else {
        return Ok(None);
    };
    transaction
        .query_row(
            "SELECT node_id FROM run_affinity_bindings \
             WHERE run_id = ?1 AND affinity_group = ?2",
            params![run_id, group],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub(in crate::daemon::run_store::scheduling) fn bind_affinity_home(
    transaction: &Transaction<'_>,
    run_id: &str,
    job: &ResolvedJob,
    node_id: &str,
    bound_at_ms: i64,
) -> Result<()> {
    let Some(affinity) = &job.affinity else {
        return Ok(());
    };
    let (group, required) = match affinity {
        Affinity::PreferSameNode { group } => (group, false),
        Affinity::RequireSameNode { group } => (group, true),
    };
    transaction.execute(
        "INSERT OR IGNORE INTO run_affinity_bindings \
         (run_id, affinity_group, node_id, bound_at_ms) VALUES (?1, ?2, ?3, ?4)",
        params![run_id, group, node_id, bound_at_ms],
    )?;
    let stored = transaction.query_row(
        "SELECT node_id FROM run_affinity_bindings WHERE run_id = ?1 AND affinity_group = ?2",
        params![run_id, group],
        |row| row.get::<_, String>(0),
    )?;
    if required && stored != node_id {
        bail!("hard affinity binding changed during reservation");
    }
    Ok(())
}
