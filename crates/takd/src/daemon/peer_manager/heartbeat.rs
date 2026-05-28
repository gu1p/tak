use prost::Message;

use super::{PeerManager, PeerState};
use crate::daemon::protocol::TorBroker;
use tak_proto::NodePingResponse;

const DEFAULT_HEARTBEAT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Clone, Debug)]
pub(super) struct HeartbeatTarget {
    pub(super) node_id: String,
    pub(super) endpoint: String,
    pub(super) bearer_token: String,
}

pub(super) fn should_ping(state: PeerState) -> bool {
    matches!(
        state,
        PeerState::Connecting
            | PeerState::Connected
            | PeerState::Degraded
            | PeerState::Unreachable
            | PeerState::ProtocolMismatch
    )
}

pub(super) async fn ping_peer(manager: &PeerManager, broker: &TorBroker, target: &HeartbeatTarget) {
    let started = std::time::Instant::now();
    let ping = broker.get_protobuf(
        &target.endpoint,
        &target.node_id,
        "/v1/node/ping",
        &target.bearer_token,
    );
    match tokio::time::timeout(heartbeat_timeout(), ping).await {
        Err(_) => manager.mark_ping_failure(&target.node_id, "ping timed out"),
        Ok(result) => handle_ping_result(manager, target, started, result),
    }
}

fn handle_ping_result(
    manager: &PeerManager,
    target: &HeartbeatTarget,
    started: std::time::Instant,
    result: anyhow::Result<(u16, Vec<u8>)>,
) {
    match result {
        Ok((200, body)) => match NodePingResponse::decode(body.as_slice()) {
            Ok(ping) => {
                manager.mark_ping_success(
                    &target.node_id,
                    ping,
                    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                );
            }
            Err(err) => {
                manager.mark_protocol_mismatch(&target.node_id, format!("invalid ping: {err:#}"));
            }
        },
        Ok((401 | 403, _)) => manager.mark_auth_failed(&target.node_id, "auth rejected"),
        Ok((404 | 501, _)) => manager.mark_protocol_mismatch(&target.node_id, "unsupported ping"),
        Ok((status, _)) => manager.mark_ping_failure(&target.node_id, format!("http {status}")),
        Err(err) => manager.mark_ping_failure(&target.node_id, format!("{err:#}")),
    }
}

fn heartbeat_timeout() -> std::time::Duration {
    std::env::var("TAKD_PEER_HEARTBEAT_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(std::time::Duration::from_millis)
        .unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT)
}

pub(super) fn unix_epoch_ms() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}
