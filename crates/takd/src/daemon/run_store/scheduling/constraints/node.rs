use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::Transaction;

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
