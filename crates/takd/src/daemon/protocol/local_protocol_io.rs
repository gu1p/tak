use super::*;
use tak_proto::local_daemon::v2::{
    DecodeOutcome, ErrorResponse as V2ErrorResponse, MAX_REQUEST_FRAME_BYTES, decode_request,
};

#[cfg(test)]
#[path = "local_protocol_io/tests.rs"]
mod tests;

pub(super) async fn handle_client(
    stream: UnixStream,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    tasks: DaemonTaskHandles,
    run_store: RunStore,
) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    loop {
        let Some(line) = read_frame(&mut reader, MAX_REQUEST_FRAME_BYTES).await? else {
            break;
        };
        if broker::is_http_request_line(&line) {
            handle_broker_http_request(&broker, &peers, line.clone(), reader, &mut writer_half)
                .await?;
            break;
        }

        let response = decode_and_dispatch_request(
            line.trim_end(),
            &manager,
            &peers,
            &broker,
            &tasks,
            &run_store,
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
    broker: &TorBroker,
    tasks: &DaemonTaskHandles,
    run_store: &RunStore,
) -> ProtocolResponse {
    match decode_request(raw_request) {
        Ok(DecodeOutcome::V2(request)) => match super::v2_dispatch::dispatch(request, run_store) {
            Ok(response) => ProtocolResponse::V2Success(response),
            Err(error) => ProtocolResponse::V2Error(error),
        },
        Ok(DecodeOutcome::LegacyCandidate) => ProtocolResponse::Legacy(
            decode_and_dispatch_legacy(raw_request, manager, peers, broker, tasks).await,
        ),
        Err(error) => ProtocolResponse::V2Error(error.into()),
    }
}

async fn decode_and_dispatch_legacy(
    raw_request: &str,
    manager: &SharedLeaseManager,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
    tasks: &DaemonTaskHandles,
) -> Response {
    let request: Request = match serde_json::from_str(raw_request) {
        Ok(request) => request,
        Err(_) => {
            tracing::debug!("invalid legacy local daemon request");
            return Response::error("unknown", "Invalid daemon request.");
        }
    };
    let request_id = legacy_request_id(&request).to_string();
    match dispatch_request(request, manager, peers, broker, tasks).await {
        Ok(response) => response,
        Err(err) => {
            tracing::error!("local daemon request failed");
            Response::error(request_id, format!("{err:#}"))
        }
    }
}

fn legacy_request_id(request: &Request) -> &str {
    match request {
        Request::AcquireLease(payload) => &payload.request_id,
        Request::RenewLease(payload) => &payload.request_id,
        Request::ReleaseLease(payload) => &payload.request_id,
        Request::Status(payload) => &payload.request_id,
        Request::PeersList(payload) => &payload.request_id,
        Request::PeersEligible(payload) => &payload.request_id,
        Request::PlaceRemote(payload) => &payload.request_id,
        Request::ForwardRemoteHttp(payload) => &payload.request_id,
        Request::StreamTaskEvents(payload) => &payload.request_id,
        Request::CancelTask(payload) => &payload.request_id,
        Request::GetTaskResult(payload) => &payload.request_id,
        Request::GetOutputRange(payload) => &payload.request_id,
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
    Legacy(Response),
    V2Success(tak_proto::local_daemon::v2::Response),
    V2Error(V2ErrorResponse),
}

impl ProtocolResponse {
    fn encode(&self) -> Result<String> {
        match self {
            Self::Legacy(response) => Ok(serde_json::to_string(response)?),
            Self::V2Success(response) => Ok(serde_json::to_string(response)?),
            Self::V2Error(response) => Ok(serde_json::to_string(response)?),
        }
    }
}
