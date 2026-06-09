use super::*;
use tak_proto::WorkspaceUploadRef;

#[path = "storage/stream.rs"]
mod stream;
pub(super) use stream::{
    commit_partial_upload, ensure_metadata, hash_partial_prefix, truncate_partial_upload,
};

/// Directory (under each remote execution root) that holds resumable workspace
/// upload blobs (`{upload_id}.zip` / `.part` / `.meta`). It is deliberately
/// excluded from the generic per-job cleanup sweep and reaped per-blob instead,
/// so a blob reused across the tasks of one job is not deleted mid-job. See the
/// cleanup janitor and `touch_upload_files`.
pub(in crate::daemon::remote) const WORKSPACE_UPLOADS_DIR_NAME: &str = ".workspace-uploads";

/// A referenced workspace upload blob is no longer present on this node (e.g. it
/// was reaped by the cleanup janitor). Distinguished from malformed-request errors
/// so the submit route can answer with a retryable status and the client can
/// re-upload instead of treating it as a hard failure.
#[derive(Debug)]
pub(in crate::daemon::remote) struct WorkspaceUploadMissing(pub String);

impl std::fmt::Display for WorkspaceUploadMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "workspace upload {} is missing", self.0)
    }
}

impl std::error::Error for WorkspaceUploadMissing {}

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
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceUploadMissing(upload.upload_id.clone()).into());
        }
        Err(err) => {
            return Err(err).with_context(|| format!("read workspace upload {}", upload.upload_id));
        }
    };
    ensure_upload_matches(&metadata, &bytes)?;
    // Refresh the blob (and its sidecars) so an actively-reused upload survives the
    // per-blob TTL sweep for as long as the job keeps referencing it. Best-effort:
    // a touch failure must not fail the resolve — the blob itself is valid.
    touch_upload_files(context, &upload.upload_id);
    Ok(bytes)
}

/// Bumps the mtime of a workspace upload's `.zip`/`.meta`/`.part` files to now.
/// Used on every successful resolve so the per-blob cleanup sweep (which keys off
/// file mtime) keeps actively-referenced blobs alive. Missing files / IO errors
/// are ignored — this is purely a liveness hint.
fn touch_upload_files(context: &RemoteNodeContext, upload_id: &str) {
    let now = std::time::SystemTime::now();
    for path in [
        completed_upload_path(context, upload_id),
        metadata_path(context, upload_id),
        partial_upload_path(context, upload_id),
    ] {
        let Ok(file) = fs::OpenOptions::new().write(true).open(&path) else {
            continue;
        };
        if let Err(err) = file.set_modified(now) {
            tracing::warn!(
                upload_id,
                path = %path.display(),
                "failed to refresh workspace upload mtime: {err}"
            );
        }
    }
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
    remote_execution_root_base(context).join(WORKSPACE_UPLOADS_DIR_NAME)
}

fn metadata_path(context: &RemoteNodeContext, upload_id: &str) -> PathBuf {
    upload_root(context).join(format!("{upload_id}.meta"))
}
