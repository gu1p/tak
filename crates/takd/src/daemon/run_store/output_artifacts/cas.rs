use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType};

use super::{CapturedFile, RunStore};

pub(super) fn capture(store: &RunStore, source: &Path) -> Result<CapturedFile> {
    let mut input = open_source(source)?;
    let metadata = input.metadata()?;
    ensure!(metadata.is_file(), "declared output is not a regular file");
    let root = output_root(store)?;
    let temporary = root.join(format!("capture-{}.tmp", uuid::Uuid::new_v4()));
    let mut output = create_private(&temporary)?;
    let mut digest = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        digest.update(&buffer[..read]);
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| anyhow::anyhow!("declared output is too large"))?;
    }
    output.sync_all()?;
    let sha256 = format!("{:x}", digest.finalize());
    fs::rename(&temporary, root.join(&sha256))?;
    File::open(&root)?.sync_all()?;
    Ok(CapturedFile {
        size,
        sha256,
        executable: executable(&metadata),
    })
}

pub(super) fn require_blob(store: &RunStore, entry: &WorkspaceEntry) -> Result<()> {
    if entry.entry_type != WorkspaceEntryType::File {
        return Ok(());
    }
    let path = blob_path(store, &entry.content_sha256);
    let mut file = open_source(&path).context("declared output blob is missing")?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.file_type().is_file() && metadata.len() == entry.size,
        "declared output blob is invalid"
    );
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == entry.content_sha256,
        "declared output blob digest is invalid"
    );
    Ok(())
}

pub(super) fn blob_path(store: &RunStore, digest: &str) -> PathBuf {
    store.blob_root.join("outputs").join(digest)
}

fn output_root(store: &RunStore) -> Result<PathBuf> {
    let root = store.blob_root.join("outputs");
    fs::create_dir_all(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    }
    Ok(root)
}

fn open_source(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .with_context(|| format!("open declared output {}", path.display()))
}

fn create_private(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(Into::into)
}

#[cfg(unix)]
fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &fs::Metadata) -> bool {
    false
}
