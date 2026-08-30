use std::path::Path;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{MAX_WORKSPACE_CHUNK_BYTES, Operation, Request, Response};

const MAX_RESYNCS: usize = 3;

pub(super) async fn workspace(
    socket_path: &Path,
    run_id: &str,
    fingerprint: &str,
    archive: &[u8],
    mut offset: u64,
) -> Result<()> {
    if offset > archive.len() as u64 {
        bail!("local takd returned an invalid workspace upload offset");
    }
    let mut resyncs = 0;
    while offset < archive.len() as u64 {
        let start = usize::try_from(offset)?;
        let end = start
            .saturating_add(MAX_WORKSPACE_CHUNK_BYTES)
            .min(archive.len());
        let response = super::exchange::response(
            socket_path,
            &Request {
                request_id: super::exchange::request_id("upload"),
                operation: Operation::UploadWorkspace {
                    run_id: run_id.to_owned(),
                    workspace_fingerprint: fingerprint.to_owned(),
                    archive_size: archive.len() as u64,
                    offset,
                    chunk: archive[start..end].to_vec(),
                },
            },
        )
        .await?;
        let Response::WorkspaceUploadProgress {
            run_id: response_run,
            workspace_fingerprint,
            chunk_accepted,
            next_offset,
            complete,
            ..
        } = response
        else {
            bail!("local takd returned an unexpected UploadWorkspace response")
        };
        if response_run != run_id
            || workspace_fingerprint != fingerprint
            || next_offset > archive.len() as u64
            || complete != (next_offset == archive.len() as u64)
        {
            bail!("local takd returned invalid workspace upload progress");
        }
        if chunk_accepted {
            if next_offset != end as u64 {
                bail!("local takd returned invalid workspace upload progress");
            }
            resyncs = 0;
        } else {
            resyncs += 1;
            if resyncs > MAX_RESYNCS {
                bail!("local takd could not resume the workspace upload");
            }
        }
        offset = next_offset;
    }
    Ok(())
}
