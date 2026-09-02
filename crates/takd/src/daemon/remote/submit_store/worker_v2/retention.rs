use std::collections::BTreeSet;

use anyhow::Result;

use rusqlite::TransactionBehavior;

use super::SubmitAttemptStore;

impl SubmitAttemptStore {
    pub(crate) fn terminal_worker_v2_runs(&self) -> Result<BTreeSet<String>> {
        let connection = self.open_connection()?;
        collect(&connection, "SELECT run_id FROM worker_v2_terminal_runs")
    }

    pub(crate) fn reclaim_terminal_worker_v2_run(
        &self,
        run_id: &str,
        reclaim: impl FnOnce() -> Result<()>,
    ) -> Result<bool> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let eligible = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM worker_v2_terminal_runs WHERE run_id=?1) AND \
             NOT EXISTS(SELECT 1 FROM worker_v2_attempts WHERE run_id=?1 AND \
             state IN ('accepted','running','cancelling'))",
            [run_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !eligible {
            return Ok(false);
        }
        reclaim()?;
        transaction.commit()?;
        Ok(true)
    }
}

fn collect(connection: &rusqlite::Connection, sql: &str) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(sql)?;
    Ok(statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}
