use super::*;

pub(in crate::daemon::remote::route_uploads) fn ensure_metadata(
    context: &RemoteNodeContext,
    upload_id: &str,
    metadata: &UploadMetadata,
) -> Result<()> {
    match read_metadata(context, upload_id) {
        Ok(existing) => {
            if existing.sha256 != metadata.sha256 || existing.size_bytes != metadata.size_bytes {
                bail!("upload metadata mismatch");
            }
            Ok(())
        }
        Err(_) => write_metadata(context, upload_id, metadata),
    }
}

pub(in crate::daemon::remote::route_uploads) fn truncate_partial_upload(
    context: &RemoteNodeContext,
    upload_id: &str,
    offset: u64,
) -> Result<()> {
    let path = partial_upload_path(context, upload_id);
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .context("open partial upload")?
        .set_len(offset)
        .context("truncate partial upload")
}

pub(in crate::daemon::remote::route_uploads) fn hash_partial_prefix(
    context: &RemoteNodeContext,
    upload_id: &str,
    len: u64,
) -> Result<Sha256> {
    use std::io::Read;

    let mut file = fs::File::open(partial_upload_path(context, upload_id))
        .context("open partial upload for hashing")?;
    let mut remaining = len;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = buffer.len().min(remaining as usize);
        let read = file
            .read(&mut buffer[..limit])
            .context("read partial upload for hashing")?;
        if read == 0 {
            bail!("partial upload shorter than requested prefix");
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(hasher)
}

pub(in crate::daemon::remote::route_uploads) fn commit_partial_upload(
    context: &RemoteNodeContext,
    upload_id: &str,
) -> Result<()> {
    let part_path = partial_upload_path(context, upload_id);
    if part_path.exists() {
        fs::rename(&part_path, completed_upload_path(context, upload_id))
            .with_context(|| format!("complete upload {upload_id}"))?;
    } else {
        fs::write(completed_upload_path(context, upload_id), [])
            .with_context(|| format!("complete upload {upload_id}"))?;
    }
    Ok(())
}
