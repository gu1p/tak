use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tak_core::remote_inventory::{RemoteInventory, load_remote_inventory_at};

#[derive(Clone)]
pub(super) struct InventoryFile {
    path: PathBuf,
}

impl InventoryFile {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn load(&self) -> Result<RemoteInventory> {
        load_remote_inventory_at(&self.path)
            .with_context(|| format!("load remote inventory {}", self.path.display()))
    }

    pub(super) fn save(&self, inventory: &RemoteInventory) -> Result<()> {
        let parent = self
            .path
            .parent()
            .context("remote inventory has no parent")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("create remote inventory directory {}", parent.display()))?;
        let temporary = temporary_path(&self.path);
        write_owner_only(&temporary, toml::to_string(inventory)?.as_bytes())?;
        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(error).with_context(|| format!("replace {}", self.path.display()));
        }
        Ok(())
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("remotes.toml");
    path.with_file_name(format!(".{name}.{}.tmp", uuid::Uuid::new_v4()))
}

#[cfg(unix)]
fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
