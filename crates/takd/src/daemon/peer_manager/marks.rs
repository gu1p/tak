use tak_proto::NodePingResponse;

use super::{HEARTBEAT_INTERVAL, PeerManager, PeerState, duration_ms, next_retry_due_ms};
use crate::daemon::peer_manager::heartbeat::unix_epoch_ms;

impl PeerManager {
    pub fn mark_ping_success(&self, node_id: &str, ping: NodePingResponse, rtt_ms: u64) {
        if ping.node_id != node_id {
            self.mark_protocol_mismatch(
                node_id,
                format!("unexpected node_id {} in ping response", ping.node_id),
            );
            return;
        }
        if ping.protocol_version != "v1" {
            self.mark_protocol_mismatch(
                node_id,
                format!("unsupported protocol version {}", ping.protocol_version),
            );
            return;
        }
        if ping.health != "healthy" {
            self.mark_ping_unhealthy(node_id, ping, rtt_ms);
            return;
        }
        self.record_healthy_ping(node_id, ping, rtt_ms);
    }

    fn record_healthy_ping(&self, node_id: &str, ping: NodePingResponse, rtt_ms: u64) {
        let mut state = self.lock_state();
        let Some(entry) = state.peers.get_mut(node_id) else {
            return;
        };
        let previous_state = entry.snapshot.state;
        let now = unix_epoch_ms();
        entry.consecutive_failures = 0;
        entry.next_heartbeat_due_ms = now.saturating_add(duration_ms(HEARTBEAT_INTERVAL));
        entry.snapshot.state = PeerState::Connected;
        entry.snapshot.last_heartbeat_ms = Some(now);
        entry.snapshot.last_successful_connection_ms = Some(now);
        entry.snapshot.last_error_summary = None;
        entry.snapshot.active_job_count = Some(ping.active_job_count);
        entry.snapshot.queue_depth = Some(ping.queue_depth);
        entry.snapshot.resource_summary = Some(ping.resource_summary);
        entry.snapshot.protocol_version = Some(ping.protocol_version);
        entry.snapshot.heartbeat_rtt_ms = Some(rtt_ms);
        entry.snapshot.reconnect_attempts = 0;
        log_peer_state_transition(entry, previous_state);
    }

    fn mark_ping_unhealthy(&self, node_id: &str, ping: NodePingResponse, rtt_ms: u64) {
        let mut state = self.lock_state();
        let Some(entry) = state.peers.get_mut(node_id) else {
            return;
        };
        let previous_state = entry.snapshot.state;
        let now = unix_epoch_ms();
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        entry.next_heartbeat_due_ms = next_retry_due_ms(
            &entry.snapshot.node_id,
            entry.snapshot.reconnect_attempts.saturating_add(1),
            now,
        );
        entry.snapshot.state = PeerState::Degraded;
        entry.snapshot.last_heartbeat_ms = Some(now);
        entry.snapshot.last_error_summary = Some(format!("peer health is {}", ping.health));
        entry.snapshot.active_job_count = Some(ping.active_job_count);
        entry.snapshot.queue_depth = Some(ping.queue_depth);
        entry.snapshot.resource_summary = Some(ping.resource_summary);
        entry.snapshot.protocol_version = Some(ping.protocol_version);
        entry.snapshot.heartbeat_rtt_ms = Some(rtt_ms);
        entry.snapshot.reconnect_attempts = entry.snapshot.reconnect_attempts.saturating_add(1);
        log_peer_state_transition(entry, previous_state);
    }

    pub fn mark_ping_failure(&self, node_id: &str, error: impl Into<String>) {
        let mut state = self.lock_state();
        let Some(entry) = state.peers.get_mut(node_id) else {
            return;
        };
        if entry.snapshot.state == PeerState::AuthFailed {
            return;
        }
        let previous_state = entry.snapshot.state;
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        entry.snapshot.last_error_summary = Some(error.into());
        entry.snapshot.reconnect_attempts = entry.snapshot.reconnect_attempts.saturating_add(1);
        entry.next_heartbeat_due_ms = next_retry_due_ms(
            &entry.snapshot.node_id,
            entry.snapshot.reconnect_attempts,
            unix_epoch_ms(),
        );
        entry.snapshot.state = match entry.snapshot.state {
            PeerState::Connected if entry.consecutive_failures == 1 => PeerState::Degraded,
            PeerState::Degraded if entry.consecutive_failures >= 2 => PeerState::Unreachable,
            PeerState::Connecting if entry.consecutive_failures >= 2 => PeerState::Unreachable,
            current => current,
        };
        log_peer_state_transition(entry, previous_state);
    }

    pub fn mark_auth_failed(&self, node_id: &str, error: impl Into<String>) {
        let mut state = self.lock_state();
        let Some(entry) = state.peers.get_mut(node_id) else {
            return;
        };
        let previous_state = entry.snapshot.state;
        entry.snapshot.state = PeerState::AuthFailed;
        entry.snapshot.last_error_summary = Some(error.into());
        entry.next_heartbeat_due_ms = i64::MAX;
        log_peer_state_transition(entry, previous_state);
    }

    pub fn mark_protocol_mismatch(&self, node_id: &str, error: impl Into<String>) {
        let mut state = self.lock_state();
        let Some(entry) = state.peers.get_mut(node_id) else {
            return;
        };
        let previous_state = entry.snapshot.state;
        entry.snapshot.state = PeerState::ProtocolMismatch;
        entry.snapshot.last_error_summary = Some(error.into());
        entry.snapshot.reconnect_attempts = entry.snapshot.reconnect_attempts.saturating_add(1);
        entry.next_heartbeat_due_ms = next_retry_due_ms(
            &entry.snapshot.node_id,
            entry.snapshot.reconnect_attempts,
            unix_epoch_ms(),
        );
        log_peer_state_transition(entry, previous_state);
    }
}

fn log_peer_state_transition(entry: &super::PeerEntry, previous_state: PeerState) {
    let next_state = entry.snapshot.state;
    if previous_state == next_state {
        return;
    }
    let node_id = entry.snapshot.node_id.as_str();
    let endpoint = entry.snapshot.endpoint.as_str();
    let detail = entry.snapshot.last_error_summary.as_deref().unwrap_or("");
    match next_state {
        PeerState::Connected => tracing::info!(
            node_id,
            endpoint,
            previous = previous_state.as_str(),
            current = next_state.as_str(),
            rtt_ms = entry.snapshot.heartbeat_rtt_ms,
            "peer state changed"
        ),
        PeerState::Degraded
        | PeerState::Unreachable
        | PeerState::AuthFailed
        | PeerState::ProtocolMismatch => tracing::warn!(
            node_id,
            endpoint,
            previous = previous_state.as_str(),
            current = next_state.as_str(),
            reconnect_attempts = entry.snapshot.reconnect_attempts,
            detail,
            "peer state changed"
        ),
        PeerState::Connecting | PeerState::Disconnected => tracing::debug!(
            node_id,
            endpoint,
            previous = previous_state.as_str(),
            current = next_state.as_str(),
            "peer state changed"
        ),
    }
}
