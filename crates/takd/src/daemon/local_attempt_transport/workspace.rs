use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::daemon::path_cache::{PathCache, Snapshot};
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};
use crate::daemon::scheduler::{AttemptCompletion, AttemptRuntimeMetadata};
use crate::daemon::workspace_layer;

mod context;
#[cfg(test)]
mod overlay_replacement_tests;
mod overlays;
mod shared;

pub(super) enum Preparation {
    Execute {
        snapshot: Box<LocalExecutionSnapshot>,
        workspace_root: PathBuf,
        path_cache: Option<(PathCache, Snapshot)>,
    },
    AlreadySettled,
    Unknown,
}

#[derive(Serialize, Deserialize)]
struct TerminalRecord {
    succeeded: bool,
    digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_engine: Option<String>,
}

pub(super) fn prepare(mut snapshot: LocalExecutionSnapshot) -> Result<Preparation> {
    if read_completion(&snapshot.attempt_root)?.is_some() {
        return Ok(Preparation::AlreadySettled);
    }
    private_dir(&snapshot.attempt_root)?;
    let started = snapshot.attempt_root.join("started");
    if started.try_exists()? {
        return Ok(Preparation::Unknown);
    }
    let (workspace_root, path_cache) = match &snapshot.workspace {
        LocalWorkspace::Private => (prepare_private(&snapshot)?, None),
        LocalWorkspace::Paths(cache) => {
            let root = prepare_private(&snapshot)?;
            let generation = cache.restore_into(&root)?;
            let cache = match std::mem::replace(&mut snapshot.workspace, LocalWorkspace::Private) {
                LocalWorkspace::Paths(cache) => cache,
                _ => unreachable!(),
            };
            (root, Some((cache, generation)))
        }
        LocalWorkspace::Shared(root) => (
            shared::prepare(&snapshot.archive_path, root, &snapshot.context_manifest)?,
            None,
        ),
    };
    overlays::apply(&workspace_root, &snapshot.overlays)?;
    Ok(Preparation::Execute {
        snapshot: Box::new(snapshot),
        workspace_root,
        path_cache,
    })
}

fn prepare_private(snapshot: &LocalExecutionSnapshot) -> Result<PathBuf> {
    let root = snapshot.attempt_root.join("workspace");
    remove_existing(&root)?;
    let base_root = local_base_root(&snapshot.archive_path)?;
    let base =
        workspace_layer::immutable_base(&base_root, |data| unpack(&snapshot.archive_path, data))?;
    workspace_layer::private_copy(&base, &root)?;
    context::filter(&root, &snapshot.context_manifest)?;
    Ok(root)
}

fn local_base_root(archive: &Path) -> Result<PathBuf> {
    let parent = archive
        .parent()
        .ok_or_else(|| anyhow::anyhow!("workspace archive has no parent"))?;
    let cache_parent = if parent.file_name().is_some_and(|name| name == "workspaces") {
        parent
            .parent()
            .ok_or_else(|| anyhow::anyhow!("workspace archive root is missing"))?
    } else {
        parent
    };
    let fingerprint = archive
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("workspace archive fingerprint is missing"))?;
    Ok(cache_parent.join("workspace-bases").join(fingerprint))
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
    let runtime = match (record.runtime_kind, record.runtime_engine) {
        (Some(kind), Some(engine)) => Some(AttemptRuntimeMetadata { kind, engine }),
        (None, None) => None,
        _ => anyhow::bail!("local terminal runtime metadata is incomplete"),
    };
    let completion = if record.succeeded {
        AttemptCompletion::Succeeded {
            terminal_digest: record.digest,
        }
    } else {
        AttemptCompletion::Failed {
            terminal_digest: record.digest,
            exit_code: record.exit_code,
        }
    };
    Ok(Some(completion.with_runtime(runtime)))
}

pub(super) fn write_completion(root: &Path, completion: &AttemptCompletion) -> Result<()> {
    let record = TerminalRecord {
        succeeded: completion.succeeded(),
        digest: completion.digest().to_owned(),
        exit_code: completion.exit_code(),
        runtime_kind: completion.runtime().map(|runtime| runtime.kind.clone()),
        runtime_engine: completion.runtime().map(|runtime| runtime.engine.clone()),
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
