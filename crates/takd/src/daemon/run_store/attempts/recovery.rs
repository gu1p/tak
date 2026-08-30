use anyhow::Result;

use crate::daemon::scheduler::DispatchCommand;

use super::super::RunStore;

impl RunStore {
    pub fn running_attempts_for_reconciliation(&self) -> Result<Vec<DispatchCommand>> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "SELECT attempt.run_id, attempt.job_id, attempt.node_id, attempt.authored_attempt, \
             attempt.dispatch_generation, attempt.fencing_token FROM run_attempts attempt \
             JOIN run_jobs job USING (run_id, job_id) JOIN runs run USING (run_id) \
             WHERE attempt.state = 'running' AND attempt.released_at_ms IS NULL \
             AND job.state = 'running' AND job.current_fencing_token = attempt.fencing_token \
             AND run.state = 'running' ORDER BY attempt.reserved_at_ms, attempt.run_id, attempt.job_id",
        )?;
        statement
            .query_map([], command_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn command_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DispatchCommand> {
    let attempt = row.get::<_, i64>(3)?;
    let generation = row.get::<_, i64>(4)?;
    Ok(DispatchCommand {
        run_id: row.get(0)?,
        job_id: row.get(1)?,
        node_id: row.get(2)?,
        authored_attempt: u32::try_from(attempt)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, attempt))?,
        dispatch_generation: u32::try_from(generation)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, generation))?,
        fencing_token: row.get(5)?,
    })
}
