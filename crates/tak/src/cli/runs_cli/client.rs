use std::path::Path;
use std::time::Duration;

use tak_proto::local_daemon::v2::{
    DaemonErrorCode, MAX_ERROR_RESPONSE_FRAME_BYTES, Request, RequestEncodeError,
    decode_error_response, encode_request,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunDaemonClientError {
    InvalidRequest,
    InvalidRunId,
    ProtocolMismatch,
    TimedOut,
    ConnectFailed,
    Disconnected,
}

pub(super) async fn send_request(
    socket_path: &Path,
    request: &Request,
) -> Result<DaemonErrorCode, RunDaemonClientError> {
    match timeout(EXCHANGE_TIMEOUT, exchange(socket_path, request)).await {
        Ok(result) => result,
        Err(_) => Err(RunDaemonClientError::TimedOut),
    }
}

async fn exchange(
    socket_path: &Path,
    request: &Request,
) -> Result<DaemonErrorCode, RunDaemonClientError> {
    let payload = encode_request(request).map_err(classify_request_error)?;
    let mut stream = UnixStream::connect(socket_path)
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
    let response = read_response_frame(&mut stream).await?;
    decode_error_response(&response, &request.request_id)
        .map_err(|_| RunDaemonClientError::ProtocolMismatch)
}

fn classify_request_error(error: RequestEncodeError) -> RunDaemonClientError {
    match error {
        RequestEncodeError::RunIdInvalid => RunDaemonClientError::InvalidRunId,
        RequestEncodeError::RequestIdInvalid | RequestEncodeError::EncodingFailed => {
            RunDaemonClientError::InvalidRequest
        }
    }
}

async fn read_response_frame(stream: &mut UnixStream) -> Result<Vec<u8>, RunDaemonClientError> {
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
            if response.len() + delimiter > MAX_ERROR_RESPONSE_FRAME_BYTES {
                return Err(RunDaemonClientError::ProtocolMismatch);
            }
            response.extend_from_slice(&bytes[..delimiter]);
            return Ok(response);
        }
        if response.len() + bytes.len() > MAX_ERROR_RESPONSE_FRAME_BYTES {
            return Err(RunDaemonClientError::ProtocolMismatch);
        }
        response.extend_from_slice(bytes);
    }
}
