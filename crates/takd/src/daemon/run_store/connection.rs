use std::fs;
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::RunStore;

impl RunStore {
    pub(super) fn open_connection(&self) -> Result<Connection> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create run-store directory {}", parent.display()))?;
            set_owner_only_dir(parent)?;
        }
        fs::create_dir_all(&self.blob_root)
            .with_context(|| format!("create blob directory {}", self.blob_root.display()))?;
        set_owner_only_dir(&self.blob_root)?;
        let connection = Connection::open(&self.db_path)
            .with_context(|| format!("open run store {}", self.db_path.display()))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;",
        )?;
        set_owner_only_file(&self.db_path)?;
        Ok(connection)
    }

    pub(super) fn upload_path(&self, run_id: &str) -> std::path::PathBuf {
        self.blob_root
            .join("uploads")
            .join(format!("{run_id}.part"))
    }

    pub(super) fn blob_path(&self, fingerprint: &str) -> std::path::PathBuf {
        self.blob_root
            .join("workspaces")
            .join(format!("{fingerprint}.tar"))
    }
}

#[cfg(unix)]
fn set_owner_only_dir(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_dir(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_file(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_file(_path: &std::path::Path) -> Result<()> {
    Ok(())
}
