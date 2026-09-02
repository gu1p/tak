use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use tak_core::v2::ResolvedRun;

mod active;
mod affinity;
mod cancellation;
mod transport;

const CURRENT_VERSION: i64 = 14;

pub(super) fn reject_newer_schema(connection: &Connection) -> Result<()> {
    let has_version_table = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'run_schema_version')",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if !has_version_table {
        return Ok(());
    }
    let version = connection
        .query_row(
            "SELECT version FROM run_schema_version WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if version.is_some_and(|version| version > CURRENT_VERSION) {
        bail!("run database was created by a newer takd; upgrade this daemon");
    }
    Ok(())
}

pub(super) fn apply(transaction: &Transaction<'_>) -> Result<()> {
    let run_options = ensure_column(
        transaction,
        "runs",
        "max_parallel_jobs",
        "max_parallel_jobs INTEGER NOT NULL DEFAULT 1",
    )? | ensure_column(
        transaction,
        "runs",
        "keep_going",
        "keep_going INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        transaction,
        "runs",
        "dispatch_stopped",
        "dispatch_stopped INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(transaction, "runs", "exit_code", "exit_code INTEGER")?;
    ensure_column(
        transaction,
        "runs",
        "logs_expired",
        "logs_expired INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        transaction,
        "runs",
        "outputs_expired",
        "outputs_expired INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(transaction, "runs", "output_error", "output_error TEXT")?;
    ensure_column(
        transaction,
        "runs",
        "last_scheduled_turn",
        "last_scheduled_turn INTEGER NOT NULL DEFAULT 0",
    )?;
    let active_pointers = ensure_column(
        transaction,
        "run_jobs",
        "dispatch_generation",
        "dispatch_generation INTEGER NOT NULL DEFAULT 0",
    )? | ensure_column(
        transaction,
        "run_jobs",
        "current_fencing_token",
        "current_fencing_token TEXT",
    )?;
    ensure_column(transaction, "run_jobs", "cache", "cache TEXT")?;
    let ready_age = ensure_column(
        transaction,
        "run_jobs",
        "next_eligible_at_ms",
        "next_eligible_at_ms INTEGER NOT NULL DEFAULT 0",
    )?;
    let ready_order = ensure_column(
        transaction,
        "run_jobs",
        "ready_order",
        "ready_order INTEGER NOT NULL DEFAULT 0",
    )?;
    for (column, declaration) in [
        ("accepted_at_ms", "accepted_at_ms INTEGER"),
        ("finished_at_ms", "finished_at_ms INTEGER"),
        ("outcome", "outcome TEXT"),
        ("terminal_digest", "terminal_digest TEXT"),
        ("exit_code", "exit_code INTEGER"),
    ] {
        ensure_column(transaction, "run_attempts", column, declaration)?;
    }
    let dispatch_started = ensure_column(
        transaction,
        "run_attempts",
        "dispatch_started_at_ms",
        "dispatch_started_at_ms INTEGER",
    )?;
    let attempt_transport =
        ensure_column(transaction, "run_attempts", "transport", "transport TEXT")?;
    ensure_column(
        transaction,
        "run_attempts",
        "worker_event_cursor",
        "worker_event_cursor INTEGER NOT NULL DEFAULT 0",
    )?;
    if run_options {
        backfill_run_options(transaction)?;
    }
    if active_pointers {
        active::backfill(transaction)?;
    }
    if attempt_transport {
        transport::backfill(transaction)?;
    }
    if dispatch_started {
        transaction.execute(
            "UPDATE run_attempts SET dispatch_started_at_ms = COALESCE(accepted_at_ms, reserved_at_ms) WHERE released_at_ms IS NULL",
            [],
        )?;
    }
    if ready_age {
        transaction.execute(
            "UPDATE run_jobs SET next_eligible_at_ms = (SELECT updated_at_ms FROM runs WHERE runs.run_id = run_jobs.run_id) WHERE state = 'ready'",
            [],
        )?;
    }
    if ready_order {
        transaction.execute(
            "UPDATE run_jobs SET ready_order = ordinal WHERE state IN ('ready', 'retrying')",
            [],
        )?;
    }
    cancellation::backfill(transaction)?;
    affinity::backfill(transaction)?;
    Ok(())
}

fn backfill_run_options(transaction: &Transaction<'_>) -> Result<()> {
    let runs = {
        let mut statement = transaction.prepare("SELECT run_id, resolved_json FROM runs")?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (run_id, resolved_json) in runs {
        let run: ResolvedRun = serde_json::from_str(&resolved_json)?;
        transaction.execute(
            "UPDATE runs SET max_parallel_jobs = ?2, keep_going = ?3 WHERE run_id = ?1",
            params![
                run_id,
                i64::from(run.options.max_parallel_jobs.get()),
                run.options.keep_going
            ],
        )?;
    }
    Ok(())
}

fn ensure_column(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<bool> {
    let mut statement = transaction.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    if columns.iter().any(|name| name == column) {
        return Ok(false);
    }
    transaction.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {declaration}"))?;
    Ok(true)
}
