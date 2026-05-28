use super::*;

use std::time::Duration;

use super::super::protocol_result_http::{
    RemoteHttpResponse, remote_protocol_http_request_with_extra_headers,
};

#[path = "remote_download/digest.rs"]
mod digest;
#[path = "remote_download/partial.rs"]
mod partial;

use digest::output_file_size_and_sha256;
use partial::{append_download_bytes, partial_download_path, partial_size, remove_partial};

const OUTPUT_DOWNLOAD_CHUNK_BYTES: u64 = 8 * 1024 * 1024;

pub(super) async fn download_remote_output(
    target: &StrictRemoteTarget,
    request_path: &str,
    destination: &Path,
    output: &SyncedOutput,
) -> Result<()> {
    let partial = partial_download_path(destination);
    for attempt in 0..3 {
        let offset = partial_size(&partial)?;
        if offset == output.size_bytes {
            return finish_download(destination, &partial, output);
        }
        let headers = range_headers(offset, output.size_bytes);
        let response = match remote_protocol_http_request_with_extra_headers(
            target,
            "GET",
            request_path,
            None,
            "outputs",
            Duration::from_secs(2),
            &headers,
        )
        .await
        {
            Ok(response) => response,
            Err(_) if attempt == 0 => continue,
            Err(err) => return Err(err.into()),
        };
        match write_download_response(&partial, offset, &response)? {
            DownloadWrite::Continue => {
                if partial_size(&partial)? >= output.size_bytes {
                    return finish_download(destination, &partial, output);
                }
            }
            DownloadWrite::Restart if attempt == 0 => {
                remove_partial(&partial)?;
                continue;
            }
            DownloadWrite::Restart => break,
        }
    }
    bail!(
        "infra error: remote node {} output download failed for {}",
        target.node_id,
        output.path
    )
}

pub(super) fn build_remote_output_request_path(
    task_run_id: &str,
    attempt: u32,
    relative_path: &Path,
) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("attempt", &attempt.to_string());
    query.append_pair("path", &normalize_filesystem_relative_path(relative_path));
    format!("/v1/tasks/{task_run_id}/outputs?{}", query.finish())
}

enum DownloadWrite {
    Continue,
    Restart,
}

fn write_download_response(
    partial: &Path,
    offset: u64,
    response: &RemoteHttpResponse,
) -> Result<DownloadWrite> {
    if response.status == 416 || (offset > 0 && response.status != 206) {
        return Ok(DownloadWrite::Restart);
    }
    if offset == 0 && response.status != 200 && response.status != 206 {
        bail!(
            "infra error: remote output download failed with HTTP {}",
            response.status
        );
    }
    if offset > 0 || response.status == 206 {
        ensure_content_range(response, offset)?;
    }
    append_download_bytes(partial, &response.body)?;
    Ok(DownloadWrite::Continue)
}

fn finish_download(destination: &Path, partial: &Path, output: &SyncedOutput) -> Result<()> {
    if let Err(err) = ensure_output_matches(partial, output) {
        let _ = remove_partial(partial);
        return Err(err);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create output sync directory {}",
                parent.to_string_lossy()
            )
        })?;
    }
    fs::rename(partial, destination).with_context(|| {
        format!(
            "failed to move remote output {} to {}",
            output.path,
            destination.display()
        )
    })
}

fn ensure_output_matches(path: &Path, output: &SyncedOutput) -> Result<()> {
    let (copied_size, actual_digest) = output_file_size_and_sha256(path)?;
    if copied_size != output.size_bytes {
        bail!(
            "infra error: remote output {} size mismatch after download (expected {}, got {})",
            output.path,
            output.size_bytes,
            copied_size
        );
    }
    let expected_digest = output
        .digest
        .strip_prefix("sha256:")
        .unwrap_or(&output.digest);
    if actual_digest != expected_digest {
        bail!(
            "infra error: remote output {} digest mismatch after download",
            output.path
        );
    }
    Ok(())
}

fn ensure_content_range(response: &RemoteHttpResponse, offset: u64) -> Result<()> {
    let Some(value) = response.header("content-range") else {
        bail!("infra error: resumed remote output missing Content-Range");
    };
    let expected = format!("bytes {offset}-");
    if !value.starts_with(&expected) {
        bail!("infra error: resumed remote output returned unexpected Content-Range {value}");
    }
    Ok(())
}

fn range_headers(offset: u64, size_bytes: u64) -> Vec<(&'static str, String)> {
    if size_bytes == 0 {
        Vec::new()
    } else {
        let end = size_bytes
            .saturating_sub(1)
            .min(offset.saturating_add(OUTPUT_DOWNLOAD_CHUNK_BYTES - 1));
        vec![("Range", format!("bytes={offset}-{end}"))]
    }
}
