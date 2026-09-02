use std::borrow::Cow;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use tak_proto::local_daemon::v2::{
    MAX_RESPONSE_FRAME_BYTES, Request, RequestEncodeError, Response, decode_response,
    encode_request,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunDaemonClientError {
    InvalidRequest,
    InvalidRunId,
    ProtocolMismatch,
    TimedOut,
    ConnectFailed,
    Disconnected,
}

pub(crate) async fn send_response(
    socket_path: &Path,
    request: &Request,
) -> Result<Response, RunDaemonClientError> {
    send_response_with_timeout(socket_path, request, EXCHANGE_TIMEOUT).await
}

pub(crate) async fn send_response_with_timeout(
    socket_path: &Path,
    request: &Request,
    deadline: Duration,
) -> Result<Response, RunDaemonClientError> {
    match timeout(deadline, exchange_response(socket_path, request)).await {
        Ok(result) => result,
        Err(_) => Err(RunDaemonClientError::TimedOut),
    }
}

async fn exchange_response(
    socket_path: &Path,
    request: &Request,
) -> Result<Response, RunDaemonClientError> {
    let payload = encode_request(request).map_err(classify_request_error)?;
    let connection_path = short_connection_path(socket_path);
    let mut stream = UnixStream::connect(connection_path.as_ref())
        .await
        .map_err(|_| RunDaemonClientError::ConnectFailed)?;
    stream
        .write_all(payload.as_bytes())
        .await
        .map_err(|_| RunDaemonClientError::Disconnected)?;
    stream
        .write_all(b"\n")
        .await
        .map_err(|_| RunDaemonClientError::Disconnected)?;
    stream
        .flush()
        .await
        .map_err(|_| RunDaemonClientError::Disconnected)?;
    let response = read_response_frame(&mut stream, MAX_RESPONSE_FRAME_BYTES).await?;
    decode_response(&response, &request.request_id)
        .map_err(|_| RunDaemonClientError::ProtocolMismatch)
}

fn short_connection_path(socket_path: &Path) -> Cow<'_, Path> {
    if !socket_path.is_absolute() || unix_path_fits(socket_path) {
        return Cow::Borrowed(socket_path);
    }
    let Ok(current_dir) = std::env::current_dir() else {
        return Cow::Borrowed(socket_path);
    };
    let Some(relative) = lexical_relative_path(socket_path, &current_dir) else {
        return Cow::Borrowed(socket_path);
    };
    if relative.as_os_str().is_empty() || !unix_path_fits(&relative) {
        return Cow::Borrowed(socket_path);
    }
    Cow::Owned(relative)
}

fn lexical_relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = target.components().collect::<Vec<_>>();
    let base = base.components().collect::<Vec<_>>();
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(target, base)| target == base)
        .count();
    if common == 0 {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in &base[common..] {
        match component {
            Component::Normal(_) => relative.push(".."),
            Component::CurDir => {}
            _ => return None,
        }
    }
    for component in &target[common..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}

fn unix_path_fits(path: &Path) -> bool {
    path.as_os_str().as_bytes().len() <= 103
}

fn classify_request_error(error: RequestEncodeError) -> RunDaemonClientError {
    match error {
        RequestEncodeError::RunIdInvalid => RunDaemonClientError::InvalidRunId,
        RequestEncodeError::RequestIdInvalid
        | RequestEncodeError::PayloadInvalid
        | RequestEncodeError::FrameTooLarge
        | RequestEncodeError::EncodingFailed => RunDaemonClientError::InvalidRequest,
    }
}

async fn read_response_frame(
    stream: &mut UnixStream,
    max_bytes: usize,
) -> Result<Vec<u8>, RunDaemonClientError> {
    let mut response = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| RunDaemonClientError::Disconnected)?;
        if read == 0 {
            return Err(RunDaemonClientError::Disconnected);
        }
        let bytes = &chunk[..read];
        if let Some(delimiter) = bytes.iter().position(|byte| *byte == b'\n') {
            if response.len() + delimiter > max_bytes {
                return Err(RunDaemonClientError::ProtocolMismatch);
            }
            response.extend_from_slice(&bytes[..delimiter]);
            return Ok(response);
        }
        if response.len() + bytes.len() > max_bytes {
            return Err(RunDaemonClientError::ProtocolMismatch);
        }
        response.extend_from_slice(bytes);
    }
}
