use anyhow::Result;
use rusqlite::Transaction;
use tak_core::v2::{ResolvedJob, SessionReuse};

pub(in crate::daemon::run_store::scheduling) fn can_acquire_shared_workspace(
    transaction: &Transaction<'_>,
    run_id: &str,
    job: &ResolvedJob,
) -> Result<bool> {
    let Some(session) = &job.session else {
        return Ok(true);
    };
    let SessionReuse::SharedWorkspace { max_parallel_tasks } = &session.reuse else {
        return Ok(true);
    };
    let definitions = {
        let mut statement = transaction.prepare(
            "SELECT job.definition_json FROM run_attempts attempt \
             JOIN run_jobs job USING (run_id, job_id) \
             WHERE attempt.run_id = ?1 AND attempt.released_at_ms IS NULL",
        )?;
        statement
            .query_map([run_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut active = 0_u32;
    for definition in definitions {
        let active_job: ResolvedJob = serde_json::from_str(&definition)?;
        if active_job
            .session
            .as_ref()
            .is_some_and(|active_session| active_session.id == session.id)
        {
            active = active
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("shared workspace usage overflow"))?;
        }
    }
    Ok(active < max_parallel_tasks.get())
}
