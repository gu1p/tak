use anyhow::Result;
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::ResolvedJob;

use crate::daemon::scheduler::DispatchCommand;

pub(in crate::daemon::run_store::attempts) struct StoredAttempt {
    pub(in crate::daemon::run_store::attempts) token: String,
    pub(in crate::daemon::run_store::attempts) node_id: String,
    pub(in crate::daemon::run_store::attempts) state: String,
    pub(in crate::daemon::run_store::attempts) outcome: Option<String>,
    pub(in crate::daemon::run_store::attempts) digest: Option<String>,
}

impl StoredAttempt {
    pub(in crate::daemon::run_store::attempts) fn matches(
        &self,
        command: &DispatchCommand,
    ) -> bool {
        self.token == command.fencing_token && self.node_id == command.node_id
    }
}

pub(in crate::daemon::run_store::attempts) fn load_attempt(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
) -> Result<Option<StoredAttempt>> {
    transaction.query_row(
        "SELECT fencing_token, node_id, state, outcome, terminal_digest FROM run_attempts WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4",
        params![command.run_id, command.job_id, command.authored_attempt, command.dispatch_generation],
        |row| Ok(StoredAttempt { token: row.get(0)?, node_id: row.get(1)?, state: row.get(2)?, outcome: row.get(3)?, digest: row.get(4)? }),
    ).optional().map_err(Into::into)
}

pub(in crate::daemon::run_store::attempts) fn load_job(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
) -> Result<ResolvedJob> {
    let definition = transaction.query_row(
        "SELECT definition_json FROM run_jobs WHERE run_id = ?1 AND job_id = ?2 AND current_fencing_token = ?3",
        params![command.run_id, command.job_id, command.fencing_token],
        |row| row.get::<_, String>(0),
    ).optional()?.ok_or_else(|| anyhow::anyhow!("attempt fence is no longer current"))?;
    Ok(serde_json::from_str(&definition)?)
}
