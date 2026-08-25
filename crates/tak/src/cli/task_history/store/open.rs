use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

use super::{TaskHistoryWriter, configure_write_connection, default_db_path, ensure_schema};

impl TaskHistoryWriter {
    pub(in crate::cli::task_history) fn open_default() -> Result<Self> {
        Self::open(default_db_path()?)
    }

    fn open(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        for attempt in 0..20 {
            match Self::open_once(&db_path) {
                Ok(writer) => return Ok(writer),
                Err(err) if attempt < 19 && is_sqlite_busy(&err) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(err) => return Err(err),
            }
        }
        unreachable!("bounded task history open retry loop returns")
    }

    fn open_once(db_path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .with_context(|| format!("open task history db {}", db_path.display()))?;
        configure_write_connection(&conn)?;
        ensure_schema(&conn)?;
        Ok(Self { conn })
    }
}

fn is_sqlite_busy(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .and_then(rusqlite::Error::sqlite_error_code)
            .is_some_and(|code| {
                matches!(
                    code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                )
            })
    })
}
