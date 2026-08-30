use super::*;
use tak_proto::local_daemon::v2::{
    DecodeOutcome, ErrorResponse as V2ErrorResponse, Operation as V2Operation,
    Request as V2Request, decode_request,
};

pub(super) async fn handle_client(
    stream: UnixStream,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    tasks: DaemonTaskHandles,
) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        // TODO: Bound frames after legacy PlaceRemote payloads no longer share this stream.
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }
        if broker::is_http_request_line(&line) {
            handle_broker_http_request(&broker, &peers, line.clone(), reader, &mut writer_half)
                .await?;
            break;
        }

        let response =
            decode_and_dispatch_request(line.trim_end(), &manager, &peers, &broker, &tasks).await;
        write_protocol_response(&mut writer_half, &response).await?;
    }

    Ok(())
}

async fn decode_and_dispatch_request(
    raw_request: &str,
    manager: &SharedLeaseManager,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
    tasks: &DaemonTaskHandles,
) -> ProtocolResponse {
    match decode_request(raw_request) {
        Ok(DecodeOutcome::V2(request)) => ProtocolResponse::V2(v2_not_active_response(request)),
        Ok(DecodeOutcome::LegacyCandidate) => ProtocolResponse::Legacy(
            decode_and_dispatch_legacy(raw_request, manager, peers, broker, tasks).await,
        ),
        Err(error) => ProtocolResponse::V2(error.into()),
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

fn v2_not_active_response(request: V2Request) -> V2ErrorResponse {
    match request.operation {
        V2Operation::ListRuns {} => tracing::debug!("recognized v2 ListRuns request"),
        V2Operation::GetRun { .. } => tracing::debug!("recognized v2 GetRun request"),
        V2Operation::AttachRun { .. } => tracing::debug!("recognized v2 AttachRun request"),
        V2Operation::CancelRun { .. } => tracing::debug!("recognized v2 CancelRun request"),
        V2Operation::GetOutputManifest { .. } => {
            tracing::debug!("recognized v2 GetOutputManifest request");
        }
    }
    V2ErrorResponse::v2_not_active(request.request_id)
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
    V2(V2ErrorResponse),
}

impl ProtocolResponse {
    fn encode(&self) -> Result<String> {
        match self {
            Self::Legacy(response) => Ok(serde_json::to_string(response)?),
            Self::V2(response) => Ok(serde_json::to_string(response)?),
        }
    }
}
