use anyhow::Result;
use rusqlite::Transaction;
use tak_core::v2::{ResolvedJob, ResolvedRun};

mod limiter;
mod model;
mod rate_limit;
#[cfg(test)]
mod rate_limit_tests;

pub(in crate::daemon::run_store::scheduling) use model::Context;
use model::{Constraint, Key, constraints};

fn queue_key(context: &Context<'_>, job: &ResolvedJob, node_id: &str) -> Result<Option<Key>> {
    Ok(constraints(context, job, node_id)?
        .into_iter()
        .find(Constraint::is_queue)
        .map(|constraint| constraint.key))
}

pub(in crate::daemon::run_store::scheduling) fn can_acquire(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<bool> {
    if !is_queue_head(transaction, context, job, node_id)? {
        return Ok(false);
    }
    if !rate_limit::can_acquire(transaction, context, job, node_id)? {
        return Ok(false);
    }
    let requested = constraints(context, job, node_id)?;
    if requested.is_empty() {
        return Ok(true);
    }
    let active = active_constraints(transaction, context.now_ms)?;
    for request in requested {
        let used = active
            .iter()
            .filter(|active| active.key == request.key)
            .try_fold(0_u64, |total, active| {
                total
                    .checked_add(active.amount)
                    .ok_or_else(|| anyhow::anyhow!("constraint usage overflow"))
            })?;
        if used
            .checked_add(request.amount)
            .is_none_or(|projected| projected > request.capacity)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(in crate::daemon::run_store::scheduling) fn consume_rate_limits(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<bool> {
    rate_limit::acquire(transaction, context, job, node_id)
}

fn is_queue_head(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<bool> {
    let Some(requested_key) = queue_key(context, job, node_id)? else {
        return Ok(true);
    };
    let rows = {
        let mut statement = transaction.prepare(
            "SELECT job.run_id, job.job_id, run.submitter_id, run.resolved_json, \
             job.definition_json FROM run_jobs job JOIN runs run USING (run_id) \
             WHERE (job.state = 'ready' OR (job.state = 'retrying' \
             AND job.next_eligible_at_ms <= ?1)) AND run.state IN ('queued', 'running') \
             AND run.dispatch_stopped = 0 ORDER BY job.next_eligible_at_ms, \
             run.created_at_ms, run.rowid, job.ready_order, job.ordinal, job.job_id",
        )?;
        statement
            .query_map([i64::try_from(context.now_ms)?], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (run_id, job_id, submitter, run, definition) in rows {
        let run: ResolvedRun = serde_json::from_str(&run)?;
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        if requested_key.is_node_scoped()
            && !job
                .placement_candidates
                .iter()
                .any(|candidate| candidate.node_id == node_id)
        {
            continue;
        }
        let other = Context {
            run_id: &run_id,
            job_id: &job_id,
            submitter_id: &submitter,
            run: &run,
            now_ms: context.now_ms,
        };
        if queue_key(&other, &job, node_id)?.as_ref() == Some(&requested_key) {
            return Ok(run_id == context.run_id && job_id == context.job_id);
        }
    }
    Ok(true)
}

fn active_constraints(transaction: &Transaction<'_>, now_ms: u64) -> Result<Vec<Constraint>> {
    let rows = {
        let mut statement = transaction.prepare(
            "SELECT attempt.run_id, attempt.node_id, attempt.reserved_at_ms, \
             attempt.accepted_at_ms, attempt.released_at_ms, run.submitter_id, \
             run.resolved_json, job.definition_json FROM run_attempts attempt \
             JOIN run_jobs job USING (run_id, job_id) JOIN runs run USING (run_id)",
        )?;
        statement
            .query_map([], |row| {
                let stored_reserved = row.get::<_, i64>(2)?;
                let reserved = u64::try_from(stored_reserved)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, stored_reserved))?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    reserved,
                    row.get::<_, Option<i64>>(3)?.is_some(),
                    row.get::<_, Option<i64>>(4)?.is_some(),
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut result = Vec::new();
    for (run_id, node_id, reserved, accepted, released, submitter, run, job) in rows {
        let run: ResolvedRun = serde_json::from_str(&run)?;
        let job: ResolvedJob = serde_json::from_str(&job)?;
        let context = Context {
            run_id: &run_id,
            job_id: &job.job_id,
            submitter_id: &submitter,
            run: &run,
            now_ms,
        };
        for mut constraint in constraints(&context, &job, &node_id)? {
            constraint.reserved_at_ms = reserved;
            constraint.accepted = accepted;
            constraint.released = released;
            if constraint.active() {
                result.push(constraint);
            }
        }
    }
    Ok(result)
}
