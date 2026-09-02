use prost::Message;

use super::{PeerManager, PeerState};
use crate::daemon::protocol::TorBroker;
use tak_proto::NodePingResponse;

// Generous enough to cover a cold onion dial plus the HTTP/2 handshake for a
// peer we have not connected to yet. Steady-state heartbeats reuse a warm,
// pooled connection and complete in well under a second, so this only bites on
// the first probe or a reconnect — and pings now run concurrently, so a slow
// peer never blocks the others.
const DEFAULT_HEARTBEAT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Clone, Debug)]
pub(super) struct HeartbeatTarget {
    pub(super) node_id: String,
    pub(super) endpoint: String,
    pub(super) bearer_token: String,
    pub(super) transport: String,
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
    let connection = crate::daemon::worker_registry::WorkerConnectionTarget {
        node_id: target.node_id.clone(),
        endpoint: target.endpoint.clone(),
        bearer_token: target.bearer_token.clone(),
        transport: target.transport.clone(),
    };
    let ping = async {
        let response = broker
            .worker_v2_http_exchange(&connection, "GET", "/v2/worker/ping", &[])
            .await?;
        Ok::<_, anyhow::Error>((response.status, response.body))
    };
    match tokio::time::timeout(heartbeat_timeout(), ping).await {
        Err(_) => {
            tracing::warn!(
                node_id = %target.node_id,
                endpoint = %target.endpoint,
                timeout_ms = heartbeat_timeout().as_millis(),
                "peer heartbeat ping timed out"
            );
            manager.mark_ping_failure(&target.node_id, "ping timed out")
        }
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
        Ok((200, body)) => match tak_proto::worker_v2::decode_display_payload(&body)
            .and_then(|payload| NodePingResponse::decode(payload.as_slice()).map_err(Into::into))
        {
            Ok(ping) => {
                let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                tracing::debug!(node_id = %target.node_id, elapsed_ms, "peer heartbeat ping ok");
                manager.mark_ping_success(&target.node_id, ping, elapsed_ms);
            }
            Err(_) => manager
                .mark_protocol_mismatch(&target.node_id, "upgrade tak, takd, and workers together"),
        },
        Ok((401 | 403, _)) => manager.mark_auth_failed(&target.node_id, "auth rejected"),
        Ok((404 | 426 | 501, _)) => manager
            .mark_protocol_mismatch(&target.node_id, "upgrade tak, takd, and workers together"),
        Ok((status, _)) => manager.mark_ping_failure(&target.node_id, format!("http {status}")),
        Err(err) => {
            tracing::warn!(node_id = %target.node_id, error = %format!("{err:#}"), "peer heartbeat ping failed");
            manager.mark_ping_failure(&target.node_id, format!("{err:#}"))
        }
    }
}

pub(super) fn heartbeat_timeout() -> std::time::Duration {
    std::env::var("TAKD_PEER_HEARTBEAT_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(std::time::Duration::from_millis)
        .unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT)
}

// How long a peer is reserved while an in-flight ping runs, so the concurrent
// heartbeat loop does not dispatch a second ping for the same peer before the
// first completes. Comfortably longer than a single ping's timeout.
pub(super) fn heartbeat_claim_window() -> std::time::Duration {
    heartbeat_timeout().saturating_add(std::time::Duration::from_secs(5))
}

pub(super) fn unix_epoch_ms() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}

pub(super) fn duration_ms(duration: std::time::Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
