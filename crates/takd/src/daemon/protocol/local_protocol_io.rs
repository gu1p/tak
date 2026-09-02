use super::*;
use tak_proto::local_daemon::v2::{
    DecodeOutcome, ErrorResponse as V2ErrorResponse, MAX_REQUEST_FRAME_BYTES,
    RequestDecodeErrorCode, decode_request,
};

#[cfg(test)]
mod tests;

pub(super) async fn handle_client(
    stream: UnixStream,
    manager: SharedLeaseManager,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    remote_access: crate::daemon::RemoteAccess,
) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    loop {
        let Some(line) = read_frame(&mut reader, MAX_REQUEST_FRAME_BYTES).await? else {
            break;
        };

        let response = decode_and_dispatch_request(
            line.trim_end(),
            &manager,
            &peers,
            &run_store,
            &remote_access,
        )
        .await;
        write_protocol_response(&mut writer_half, &response).await?;
    }

    Ok(())
}

async fn read_frame<R>(reader: &mut R, max_payload_bytes: usize) -> std::io::Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    let mut frame = Vec::with_capacity(max_payload_bytes.min(4096));
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            return String::from_utf8(frame).map(Some).map_err(invalid_utf8);
        }
        let delimiter = available.iter().position(|byte| *byte == b'\n');
        let payload_bytes = delimiter.unwrap_or(available.len());
        if frame.len().saturating_add(payload_bytes) > max_payload_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "local daemon protocol frame exceeds the byte limit",
            ));
        }
        let consumed = delimiter.map_or(payload_bytes, |position| position + 1);
        frame.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if delimiter.is_some() {
            return String::from_utf8(frame).map(Some).map_err(invalid_utf8);
        }
    }
}

fn invalid_utf8(_error: std::string::FromUtf8Error) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "local daemon protocol frame is not UTF-8",
    )
}

async fn decode_and_dispatch_request(
    raw_request: &str,
    manager: &SharedLeaseManager,
    peers: &crate::daemon::peer_manager::PeerManager,
    run_store: &RunStore,
    remote_access: &crate::daemon::RemoteAccess,
) -> ProtocolResponse {
    match decode_request(raw_request) {
        Ok(DecodeOutcome::V2(request)) => {
            match super::v2_dispatch::dispatch(request, manager, run_store, peers, remote_access)
                .await
            {
                Ok(response) => ProtocolResponse::V2Success(response),
                Err(error) => ProtocolResponse::V2Error(error),
            }
        }
        Err(error) => {
            if error.code == RequestDecodeErrorCode::VersionUnsupported {
                tracing::warn!("invalid legacy local daemon request; protocol v2 is required");
            }
            ProtocolResponse::V2Error(error.into())
        }
    }
}

async fn write_protocol_response(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    response: &ProtocolResponse,
) -> Result<()> {
    let encoded = response.encode()?;
    writer.write_all(encoded.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

enum ProtocolResponse {
    V2Success(tak_proto::local_daemon::v2::Response),
    V2Error(V2ErrorResponse),
}

impl ProtocolResponse {
    fn encode(&self) -> Result<String> {
        match self {
            Self::V2Success(response) => Ok(serde_json::to_string(response)?),
            Self::V2Error(response) => Ok(serde_json::to_string(response)?),
        }
    }
}
