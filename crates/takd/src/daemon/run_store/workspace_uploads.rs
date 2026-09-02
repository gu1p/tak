use anyhow::{Result, bail};
use rusqlite::Transaction;
use tak_core::v2::WorkspaceDescriptor;
use tak_proto::local_daemon::v2::WorkspaceDisposition;

use super::RunStore;
use super::blob::verified_blob;

mod files;
mod state;

pub(super) use files::{append, remove};
pub(super) use state::{record_progress, release_if_unused, reset};

pub(super) struct SharedUpload {
    pub(super) owner_run_id: String,
    pub(super) archive_sha256: String,
    pub(super) archive_size: u64,
    pub(super) next_offset: u64,
}

pub(super) fn disposition(
    store: &RunStore,
    transaction: &Transaction<'_>,
    run_id: &str,
    descriptor: &WorkspaceDescriptor,
    fallback_offset: u64,
) -> Result<WorkspaceDisposition> {
    let fingerprint = &descriptor.manifest.fingerprint;
    if verified_blob(store, transaction, fingerprint)?.is_some() {
        discard_present(store, transaction, fingerprint)?;
        return Ok(WorkspaceDisposition::Present);
    }
    let upload = resolve(store, transaction, run_id, descriptor, fallback_offset)?;
    Ok(WorkspaceDisposition::UploadRequired {
        next_offset: upload.next_offset,
    })
}

pub(super) fn resolve(
    store: &RunStore,
    transaction: &Transaction<'_>,
    run_id: &str,
    descriptor: &WorkspaceDescriptor,
    fallback_offset: u64,
) -> Result<SharedUpload> {
    let mut upload = state::load_or_claim(transaction, run_id, descriptor, fallback_offset)?;
    validate_descriptor(&upload, descriptor)?;
    let path = store.upload_path(&upload.owner_run_id);
    if !files::matches_offset(&path, upload.next_offset)? {
        state::reset(transaction, &descriptor.manifest.fingerprint)?;
        files::remove(&path);
        upload.next_offset = 0;
    }
    state::sync_waiters(
        transaction,
        &descriptor.manifest.fingerprint,
        upload.next_offset,
    )?;
    Ok(upload)
}

fn validate_descriptor(upload: &SharedUpload, descriptor: &WorkspaceDescriptor) -> Result<()> {
    if upload.archive_sha256 != descriptor.archive_sha256
        || upload.archive_size != descriptor.archive_size
    {
        bail!("workspace upload descriptor conflicts with an active upload");
    }
    Ok(())
}

fn discard_present(
    store: &RunStore,
    transaction: &Transaction<'_>,
    fingerprint: &str,
) -> Result<()> {
    let owner = state::discard(transaction, fingerprint)?;
    if let Some(owner) = owner {
        files::remove(&store.upload_path(&owner));
    }
    Ok(())
}
