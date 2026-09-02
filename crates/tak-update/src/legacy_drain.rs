//! Read-only upgrade guard for unfinished protocol-v1 worker attempts.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, params};

const LEGACY_DB_NAME: &str = "agent.sqlite";

/// Refuses binary replacement while the worker's legacy attempt store is active.
///
/// ```no_run
/// # // Reason: reads a daemon-owned SQLite store from the local filesystem.
/// # fn main() -> anyhow::Result<()> {
/// tak_update::legacy_drain::ensure_legacy_attempts_drained(
///     std::path::Path::new("/var/lib/takd"),
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn ensure_legacy_attempts_drained(state_root: &Path) -> Result<()> {
    let db_path = state_root.join(LEGACY_DB_NAME);
    if !db_path.exists() {
        return Ok(());
    }
    let connection = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("inspect legacy attempt store {}", db_path.display()))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .context("configure legacy attempt store timeout")?;
    let active = active_attempt_count(&connection)?;
    if active > 0 {
        bail!(
            "{active} active legacy v1 attempt(s) remain; active legacy attempts must finish before replacing tak/takd binaries"
        );
    }
    Ok(())
}

fn active_attempt_count(connection: &Connection) -> Result<i64> {
    if !table_exists(connection, "submit_attempts")? {
        return Ok(0);
    }
    let sql = if table_exists(connection, "submit_results")? {
        "SELECT COUNT(*) FROM submit_attempts attempts
         LEFT JOIN submit_results results
           ON attempts.idempotency_key = results.idempotency_key
         WHERE results.idempotency_key IS NULL"
    } else {
        "SELECT COUNT(*) FROM submit_attempts"
    };
    connection
        .query_row(sql, [], |row| row.get(0))
        .context("count active legacy attempts")
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![table],
            |row| row.get(0),
        )
        .context("inspect legacy attempt schema")
}
