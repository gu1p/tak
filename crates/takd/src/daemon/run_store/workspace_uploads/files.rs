use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Result, bail};

pub(in crate::daemon::run_store) fn append(path: &Path, offset: u64, chunk: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    if file.metadata()?.len() < offset {
        bail!("workspace upload file is shorter than its durable offset");
    }
    file.set_len(offset)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(chunk)?;
    file.sync_data()?;
    Ok(())
}

pub(in crate::daemon::run_store) fn matches_offset(path: &Path, offset: u64) -> Result<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() && metadata.len() == offset),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(offset == 0),
        Err(error) => Err(error.into()),
    }
}

pub(in crate::daemon::run_store) fn remove(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("could not remove v2 workspace upload: {error}");
    }
}
