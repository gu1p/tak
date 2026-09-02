use std::collections::BTreeMap;
use std::fs::{self, File, FileTimes, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceDescriptor;

static TRANSFERS: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

pub(super) fn transfer_lock(path: &Path) -> Result<Arc<Mutex<()>>> {
    let mut locks = TRANSFERS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| anyhow::anyhow!("worker workspace lock registry poisoned"))?;
    Ok(locks
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

pub(super) fn write_and_publish(
    temporary: &Path,
    destination: &Path,
    archive: &[u8],
) -> Result<()> {
    let _ = fs::remove_file(temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    file.write_all(archive)?;
    file.sync_all()?;
    fs::rename(temporary, destination)?;
    File::open(destination.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()?;
    Ok(())
}

pub(super) fn touch(path: &Path) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .open(path)?
        .set_times(FileTimes::new().set_modified(SystemTime::now()))?;
    Ok(())
}

pub(super) fn verify(descriptor: &WorkspaceDescriptor, archive: &[u8]) -> Result<()> {
    if archive.len() as u64 != descriptor.archive_size
        || format!("{:x}", Sha256::digest(archive)) != descriptor.archive_sha256
    {
        bail!("worker workspace cache blob digest mismatch");
    }
    Ok(())
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
