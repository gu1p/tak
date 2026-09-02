use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceManifest;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CheckoutContext {
    pub(super) root: PathBuf,
    pub(super) submitted_manifest: WorkspaceManifest,
}

impl CheckoutContext {
    pub(super) fn new(root: &Path, submitted_manifest: WorkspaceManifest) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("resolve checkout root {}", root.display()))?;
        ensure!(root.is_dir(), "checkout root is not a directory");
        Ok(Self {
            root,
            submitted_manifest,
        })
    }
}

#[derive(Clone)]
pub(super) struct RunCheckoutStore {
    root: PathBuf,
}

impl RunCheckoutStore {
    pub(super) fn open_default() -> Result<Self> {
        Ok(Self::at(state_home()?.join("tak/run-checkouts")))
    }

    pub(super) fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub(super) fn record(
        &self,
        socket: &Path,
        run_id: &str,
        context: &CheckoutContext,
    ) -> Result<()> {
        private_dir(&self.root)?;
        let directory = self.root.join(socket_key(socket)?);
        private_dir(&directory)?;
        let destination = directory.join(record_name(run_id));
        if let Some(existing) = read(&destination)? {
            return same_or_conflict(existing, run_id, context);
        }
        let temporary = directory.join(format!("record-{}.tmp", uuid::Uuid::new_v4()));
        write_record(&temporary, run_id, context)?;
        match fs::hard_link(&temporary, &destination) {
            Ok(()) => {
                fs::remove_file(&temporary)?;
                fs::File::open(&directory)?.sync_all()?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                fs::remove_file(&temporary)?;
                let existing = read(&destination)?
                    .ok_or_else(|| anyhow!("checkout association disappeared"))?;
                same_or_conflict(existing, run_id, context)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error.into())
            }
        }
    }

    pub(super) fn load(&self, socket: &Path, run_id: &str) -> Result<Option<CheckoutContext>> {
        let path = self
            .root
            .join(socket_key(socket)?)
            .join(record_name(run_id));
        let Some(stored) = read(&path)? else {
            return Ok(None);
        };
        ensure!(
            stored.run_id == run_id,
            "checkout association identity mismatch"
        );
        Ok(Some(stored.context))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCheckout {
    run_id: String,
    context: CheckoutContext,
}

fn same_or_conflict(stored: StoredCheckout, run_id: &str, context: &CheckoutContext) -> Result<()> {
    ensure!(
        stored.run_id == run_id,
        "checkout association identity mismatch"
    );
    if stored.context != *context {
        bail!("run is already associated with a different checkout or submitted snapshot");
    }
    Ok(())
}

fn write_record(path: &Path, run_id: &str, context: &CheckoutContext) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&serde_json::to_vec(&StoredCheckout {
        run_id: run_id.to_owned(),
        context: context.clone(),
    })?)?;
    file.sync_all()?;
    Ok(())
}

fn read(path: &Path) -> Result<Option<StoredCheckout>> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        file.metadata()?.is_file(),
        "checkout association is not a file"
    );
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Some(
        serde_json::from_slice(&bytes).context("decode checkout association")?,
    ))
}

fn socket_key(socket: &Path) -> Result<String> {
    let absolute = if socket.is_absolute() {
        socket.to_owned()
    } else {
        std::env::current_dir()?.join(socket)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| anyhow!("daemon socket has no parent"))?;
    let parent = parent.canonicalize().unwrap_or_else(|_| parent.to_owned());
    let identity = parent.join(
        absolute
            .file_name()
            .ok_or_else(|| anyhow!("invalid daemon socket"))?,
    );
    Ok(format!(
        "{:x}",
        Sha256::digest(identity.as_os_str().as_encoded_bytes())
    ))
}

fn record_name(run_id: &str) -> String {
    format!("{:x}.json", Sha256::digest(run_id.as_bytes()))
}

fn state_home() -> Result<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| anyhow!("failed to resolve xdg_state_home"))
}

fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    ensure!(
        fs::symlink_metadata(path)?.is_dir(),
        "checkout association path is not a directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
