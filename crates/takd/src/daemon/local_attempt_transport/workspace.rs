use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};
use crate::daemon::scheduler::AttemptCompletion;

mod shared;

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
    let workspace_root = match &snapshot.workspace {
        LocalWorkspace::Private => prepare_private(&snapshot)?,
        LocalWorkspace::Shared(root) => shared::prepare(&snapshot.archive_path, root)?,
    };
    Ok(Preparation::Execute {
        snapshot,
        workspace_root,
    })
}

fn prepare_private(snapshot: &LocalExecutionSnapshot) -> Result<PathBuf> {
    let root = snapshot.attempt_root.join("workspace");
    remove_existing(&root)?;
    private_dir(&root)?;
    unpack(&snapshot.archive_path, &root)?;
    Ok(root)
}

fn unpack(archive_path: &Path, destination: &Path) -> Result<()> {
    let archive = fs::File::open(archive_path).context("open verified workspace blob")?;
    tar::Archive::new(archive)
        .unpack(destination)
        .context("unpack verified workspace blob")
}

fn remove_existing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?;
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub(super) fn mark_started(root: &Path) -> Result<()> {
    let path = root.join("started");
    let mut file = create_private(&path)?;
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
    let mut file = create_private(&temporary)?;
    file.write_all(&serde_json::to_vec(&record)?)?;
    file.sync_all()?;
    fs::rename(temporary, root.join("terminal.json"))?;
    fs::File::open(root)?.sync_all()?;
    Ok(())
}

fn create_private(path: &Path) -> Result<fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let file = options.open(path)?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        Ok(options.open(path)?)
    }
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
