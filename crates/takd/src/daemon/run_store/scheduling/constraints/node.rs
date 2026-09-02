use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::Transaction;

use crate::daemon::scheduler::SchedulerNode;

pub(in crate::daemon::run_store::scheduling) fn restore_healthy_nodes(
    transaction: &Transaction<'_>,
    nodes: &[SchedulerNode],
) -> Result<()> {
    for node in nodes {
        transaction.execute(
            "DELETE FROM scheduler_node_losses WHERE node_id = ?1",
            [&node.node_id],
        )?;
    }
    Ok(())
}

pub(in crate::daemon::run_store::scheduling) fn lost_nodes(
    transaction: &Transaction<'_>,
) -> Result<BTreeSet<String>> {
    let mut statement =
        transaction.prepare("SELECT node_id FROM scheduler_node_losses ORDER BY node_id")?;
    statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()
        .map_err(Into::into)
}
