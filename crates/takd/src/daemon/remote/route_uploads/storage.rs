use super::*;
use tak_proto::WorkspaceUploadRef;

#[path = "storage/stream.rs"]
mod stream;
pub(super) use stream::{
    commit_partial_upload, ensure_metadata, hash_partial_prefix, truncate_partial_upload,
};

pub(super) struct UploadStatus {
    pub(super) offset: u64,
    pub(super) complete: bool,
}

pub(super) struct UploadMetadata {
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

impl UploadMetadata {
    pub(super) fn from_begin(request: &BeginWorkspaceUploadRequest) -> Result<Self> {
        let sha256 = request.sha256.trim().to_ascii_lowercase();
        if sha256.len() != 64 || !sha256.chars().all(|value| value.is_ascii_hexdigit()) {
            bail!("invalid upload sha256");
        }
        Ok(Self {
            sha256,
            size_bytes: request.size_bytes,
        })
    }
}

pub(in crate::daemon::remote) fn resolve_workspace_upload_zip(
    context: &RemoteNodeContext,
    upload: &WorkspaceUploadRef,
) -> Result<Vec<u8>> {
    ensure_valid_upload_id(&upload.upload_id)?;
    let metadata = UploadMetadata {
        sha256: upload.sha256.clone(),
        size_bytes: upload.size_bytes,
    };
    let path = completed_upload_path(context, &upload.upload_id);
    let bytes = fs::read(&path)
        .with_context(|| format!("workspace upload {} is not complete", upload.upload_id))?;
    ensure_upload_matches(&metadata, &bytes)?;
    Ok(bytes)
}

pub(super) fn ensure_upload_root(context: &RemoteNodeContext) -> Result<()> {
    fs::create_dir_all(upload_root(context)).context("create workspace upload root")
}

pub(super) fn upload_status(
    context: &RemoteNodeContext,
    upload_id: &str,
    metadata: &UploadMetadata,
) -> Result<UploadStatus> {
    if let Ok(bytes) = fs::read(completed_upload_path(context, upload_id)) {
        ensure_upload_matches(metadata, &bytes)?;
        return Ok(UploadStatus {
            offset: metadata.size_bytes,
            complete: true,
        });
    }
    Ok(UploadStatus {
        offset: current_upload_offset(context, upload_id)?,
        complete: false,
    })
}

pub(super) fn write_metadata(
    context: &RemoteNodeContext,
    upload_id: &str,
    metadata: &UploadMetadata,
) -> Result<()> {
    let path = metadata_path(context, upload_id);
    let body = format!("{}\n{}\n", metadata.sha256, metadata.size_bytes);
    fs::write(path, body).context("write upload metadata")
}

pub(super) fn read_metadata(
    context: &RemoteNodeContext,
    upload_id: &str,
) -> Result<UploadMetadata> {
    let body =
        fs::read_to_string(metadata_path(context, upload_id)).context("read upload metadata")?;
    let mut lines = body.lines();
    let sha256 = lines.next().unwrap_or_default().to_string();
    let size_bytes = lines
        .next()
        .unwrap_or_default()
        .parse::<u64>()
        .context("parse upload size")?;
    Ok(UploadMetadata { sha256, size_bytes })
}

pub(super) fn current_upload_offset(context: &RemoteNodeContext, upload_id: &str) -> Result<u64> {
    let path = partial_upload_path(context, upload_id);
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(err).context("stat partial upload"),
    }
}

pub(super) fn append_chunk(
    context: &RemoteNodeContext,
    upload_id: &str,
    chunk: &[u8],
) -> Result<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(partial_upload_path(context, upload_id))
        .context("open partial upload")?;
    file.write_all(chunk).context("append upload chunk")
}

pub(super) fn ensure_upload_matches(metadata: &UploadMetadata, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != metadata.size_bytes {
        bail!("upload size mismatch");
    }
    let digest = format!("{:x}", Sha256::digest(bytes));
    if digest != metadata.sha256 {
        bail!("upload digest mismatch");
    }
    Ok(())
}

pub(super) fn partial_upload_path(context: &RemoteNodeContext, upload_id: &str) -> PathBuf {
    upload_root(context).join(format!("{upload_id}.part"))
}

pub(super) fn completed_upload_path(context: &RemoteNodeContext, upload_id: &str) -> PathBuf {
    upload_root(context).join(format!("{upload_id}.zip"))
}

pub(super) fn ensure_valid_upload_id(upload_id: &str) -> Result<()> {
    if upload_id.is_empty()
        || !upload_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
    {
        bail!("invalid upload id");
    }
    Ok(())
}

fn upload_root(context: &RemoteNodeContext) -> PathBuf {
    remote_execution_root_base(context).join(".workspace-uploads")
}

fn metadata_path(context: &RemoteNodeContext, upload_id: &str) -> PathBuf {
    upload_root(context).join(format!("{upload_id}.meta"))
}
