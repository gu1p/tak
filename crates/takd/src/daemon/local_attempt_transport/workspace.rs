use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::daemon::run_store::execution::LocalExecutionSnapshot;
use crate::daemon::scheduler::AttemptCompletion;

pub(super) enum Preparation {
    Execute {
        snapshot: LocalExecutionSnapshot,
        workspace_root: PathBuf,
    },
    AlreadySettled,
    Unknown,
}

#[derive(Serialize, Deserialize)]
struct TerminalRecord {
    succeeded: bool,
    digest: String,
}

pub(super) fn prepare(snapshot: LocalExecutionSnapshot) -> Result<Preparation> {
    if read_completion(&snapshot.attempt_root)?.is_some() {
        return Ok(Preparation::AlreadySettled);
    }
    private_dir(&snapshot.attempt_root)?;
    let started = snapshot.attempt_root.join("started");
    if started.try_exists()? {
        return Ok(Preparation::Unknown);
    }
    let workspace_root = snapshot.attempt_root.join("workspace");
    private_dir(&workspace_root)?;
    let archive = fs::File::open(&snapshot.archive_path).context("open verified workspace blob")?;
    tar::Archive::new(archive)
        .unpack(&workspace_root)
        .context("unpack verified workspace blob")?;
    Ok(Preparation::Execute {
        snapshot,
        workspace_root,
    })
}

pub(super) fn mark_started(root: &Path) -> Result<()> {
    let path = root.join("started");
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(b"v2\n")?;
    file.sync_all()?;
    fs::File::open(root)?.sync_all()?;
    Ok(())
}

pub(super) fn read_completion(root: &Path) -> Result<Option<AttemptCompletion>> {
    let path = root.join("terminal.json");
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let record: TerminalRecord = serde_json::from_slice(&bytes)?;
    Ok(Some(if record.succeeded {
        AttemptCompletion::Succeeded {
            terminal_digest: record.digest,
        }
    } else {
        AttemptCompletion::Failed {
            terminal_digest: record.digest,
        }
    }))
}

pub(super) fn write_completion(root: &Path, completion: &AttemptCompletion) -> Result<()> {
    let record = match completion {
        AttemptCompletion::Succeeded { terminal_digest } => TerminalRecord {
            succeeded: true,
            digest: terminal_digest.clone(),
        },
        AttemptCompletion::Failed { terminal_digest } => TerminalRecord {
            succeeded: false,
            digest: terminal_digest.clone(),
        },
    };
    let temporary = root.join(format!("terminal-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec(&record)?)?;
    file.sync_all()?;
    fs::rename(temporary, root.join("terminal.json"))?;
    fs::File::open(root)?.sync_all()?;
    Ok(())
}

fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
