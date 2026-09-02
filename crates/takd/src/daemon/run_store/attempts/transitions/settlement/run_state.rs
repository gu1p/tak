use anyhow::Result;
use rusqlite::{OptionalExtension, Transaction, params};
use tak_proto::local_daemon::v2::RunEventKind;

use super::super::super::super::events::{append_terminal_event, now_ms, sqlite_i64};
use super::super::super::super::output_artifacts::{self, FinalPublication};

pub(in crate::daemon::run_store::attempts) fn refresh_run_state(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<()> {
    let (nonterminal, failed, active) = transaction.query_row(
        "SELECT SUM(CASE WHEN state NOT IN ('succeeded','failed','cancelled','skipped') THEN 1 ELSE 0 END), SUM(CASE WHEN state IN ('failed','skipped') THEN 1 ELSE 0 END), SUM(CASE WHEN state IN ('transferring','running','output_committing','cancelling') THEN 1 ELSE 0 END) FROM run_jobs WHERE run_id = ?1",
        [run_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    )?;
    let state = if nonterminal == 0 {
        if failed == 0 { "succeeded" } else { "failed" }
    } else if active > 0 {
        "running"
    } else {
        "queued"
    };
    let previous =
        transaction.query_row("SELECT state FROM runs WHERE run_id=?1", [run_id], |row| {
            row.get::<_, String>(0)
        })?;
    let exit_code = terminal_exit_code(transaction, run_id, state)?;
    transaction.execute(
        "UPDATE runs SET state=?2,updated_at_ms=?3,exit_code=?4 WHERE run_id=?1",
        params![
            run_id,
            state,
            sqlite_i64(now_ms()?, "timestamp")?,
            exit_code
        ],
    )?;
    if previous != state && state == "failed" {
        publish_failed_run_outputs(transaction, run_id)?;
    }
    if previous != state && matches!(state, "succeeded" | "failed") {
        super::super::super::super::remote_attempts::rearm_terminal_run(transaction, run_id)?;
        append_terminal_event(
            transaction,
            run_id,
            if state == "succeeded" {
                RunEventKind::Succeeded
            } else {
                RunEventKind::Failed
            },
            if state == "succeeded" {
                "run succeeded"
            } else {
                "run failed"
            },
            exit_code,
        )?;
    }
    Ok(())
}

fn publish_failed_run_outputs(transaction: &Transaction<'_>, run_id: &str) -> Result<()> {
    match output_artifacts::publish_final(transaction, run_id, None)? {
        FinalPublication::Published | FinalPublication::Conflict(_) => Ok(()),
    }
}

fn terminal_exit_code(
    transaction: &Transaction<'_>,
    run_id: &str,
    state: &str,
) -> Result<Option<i32>> {
    if state == "succeeded" {
        return Ok(Some(0));
    }
    if state != "failed" {
        return Ok(None);
    }
    transaction
        .query_row(
            "SELECT attempt.exit_code FROM run_attempts attempt JOIN run_jobs job USING (run_id,job_id) WHERE attempt.run_id=?1 AND attempt.outcome='failed' AND attempt.exit_code IS NOT NULL ORDER BY job.ordinal,attempt.authored_attempt DESC,attempt.dispatch_generation DESC LIMIT 1",
            [run_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}
