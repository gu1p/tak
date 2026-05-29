use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::daemon::protocol::TorBroker;

mod eligibility;
mod heartbeat;
mod inventory;
mod marks;
mod placement;
mod reconcile;
mod state;

pub use eligibility::{PeerEligibility, first_eligible_or_error, first_placeable_or_error};
use eligibility::{peer_is_eligible, peer_is_placeable};
use heartbeat::{HeartbeatTarget, ping_peer, should_ping, unix_epoch_ms};
pub use placement::{PeerPlacementRequest, PeerPlacementSelection};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const HEARTBEAT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const RECONNECT_BACKOFF_INITIAL_MS: i64 = 1_000;
const RECONNECT_BACKOFF_MAX_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerState {
    Disconnected,
    Connecting,
    Connected,
    Degraded,
    AuthFailed,
    Unreachable,
    ProtocolMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub node_id: String,
    pub display_name: String,
    pub transport: String,
    pub endpoint: String,
    pub state: PeerState,
    pub last_heartbeat_ms: Option<i64>,
    pub last_successful_connection_ms: Option<i64>,
    pub last_error_summary: Option<String>,
    pub active_job_count: Option<u32>,
    pub queue_depth: Option<u32>,
    pub resource_summary: Option<String>,
    pub protocol_version: Option<String>,
    pub heartbeat_rtt_ms: Option<u64>,
    pub reconnect_attempts: u32,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerConnectionTarget {
    pub node_id: String,
    pub endpoint: String,
    pub bearer_token: String,
}

#[derive(Clone, Default)]
pub struct PeerManager {
    inner: Arc<Mutex<PeerManagerState>>,
}

#[derive(Default)]
struct PeerManagerState {
    peers: BTreeMap<String, PeerEntry>,
    placement_assignments: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
struct PeerEntry {
    snapshot: PeerSnapshot,
    bearer_token: String,
    consecutive_failures: u32,
    next_heartbeat_due_ms: i64,
}

impl PeerManager {
    pub fn snapshots(&self) -> Vec<PeerSnapshot> {
        let state = self.lock_state();
        state
            .peers
            .values()
            .map(|entry| entry.snapshot.clone())
            .collect()
    }

    pub fn eligible(&self, requirements: &PeerEligibility) -> Vec<PeerSnapshot> {
        self.snapshots()
            .into_iter()
            .filter(|snapshot| peer_is_eligible(snapshot, requirements))
            .collect()
    }

    pub fn placeable(&self, requirements: &PeerEligibility) -> Vec<PeerSnapshot> {
        self.snapshots()
            .into_iter()
            .filter(|snapshot| peer_is_placeable(snapshot, requirements))
            .collect()
    }

    pub fn connection_target(&self, node_id: &str) -> Option<PeerConnectionTarget> {
        let state = self.lock_state();
        state.peers.get(node_id).map(PeerEntry::connection_target)
    }

    // Selects the peers whose heartbeat is due and immediately reserves them by
    // pushing their next-due time past the claim window. This lets the loop
    // dispatch pings concurrently without a slow or hung peer being re-selected
    // (and re-pinged) on the next 1s poll while its ping is still in flight.
    // Each ping rewrites the real next-due time when it finishes.
    fn claim_heartbeat_targets(&self, now_ms: i64, claim_ms: i64) -> Vec<HeartbeatTarget> {
        let claim_until = now_ms.saturating_add(claim_ms);
        let mut state = self.lock_state();
        let mut targets = Vec::new();
        for entry in state.peers.values_mut() {
            if should_ping(entry.snapshot.state) && entry.next_heartbeat_due_ms <= now_ms {
                entry.next_heartbeat_due_ms = claim_until;
                targets.push(HeartbeatTarget {
                    node_id: entry.snapshot.node_id.clone(),
                    endpoint: entry.snapshot.endpoint.clone(),
                    bearer_token: entry.bearer_token.clone(),
                });
            }
        }
        targets
    }

    pub fn spawn_heartbeat_loop(&self, broker: TorBroker) {
        let manager = self.clone();
        tokio::spawn(async move {
            let claim_ms = duration_ms(heartbeat::heartbeat_claim_window());
            loop {
                for target in manager.claim_heartbeat_targets(unix_epoch_ms(), claim_ms) {
                    // Ping peers concurrently so one slow onion dial cannot stall
                    // heartbeats for every other peer. The claim above prevents a
                    // duplicate ping for this peer until the spawned task finishes.
                    let manager = manager.clone();
                    let broker = broker.clone();
                    tokio::spawn(async move {
                        ping_peer(&manager, &broker, &target).await;
                    });
                }
                tokio::time::sleep(HEARTBEAT_POLL_INTERVAL).await;
            }
        });
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, PeerManagerState> {
        self.inner.lock().expect("peer manager lock poisoned")
    }
}

impl PeerEntry {
    fn connection_target(&self) -> PeerConnectionTarget {
        PeerConnectionTarget {
            node_id: self.snapshot.node_id.clone(),
            endpoint: self.snapshot.endpoint.clone(),
            bearer_token: self.bearer_token.clone(),
        }
    }
}

fn next_retry_due_ms(node_id: &str, attempts: u32, now_ms: i64) -> i64 {
    now_ms.saturating_add(backoff_delay_ms(node_id, attempts))
}

fn backoff_delay_ms(node_id: &str, attempts: u32) -> i64 {
    let exponent = attempts.saturating_sub(1).min(16);
    let base = RECONNECT_BACKOFF_INITIAL_MS
        .saturating_mul(1_i64 << exponent)
        .min(RECONNECT_BACKOFF_MAX_MS);
    apply_jitter(base, node_id, attempts)
}

fn apply_jitter(base_ms: i64, node_id: &str, attempts: u32) -> i64 {
    let mut hash = u64::from(attempts);
    for byte in node_id.as_bytes() {
        hash = hash
            .wrapping_mul(1099511628211)
            .wrapping_add(u64::from(*byte));
    }
    let jitter_percent = i64::try_from(hash % 41).unwrap_or(0) - 20;
    base_ms.saturating_mul(100 + jitter_percent) / 100
}

fn duration_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
