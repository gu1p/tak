use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::{PeerSnapshot, PeerState};

mod capacity_diagnostic;
mod resource_summary;

use capacity_diagnostic::{capacity_diagnostic, has_resource_requirements};
use resource_summary::PeerResources;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PeerEligibility {
    pub pool: Option<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: Option<String>,
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
}

pub(super) fn peer_is_eligible(snapshot: &PeerSnapshot, requirements: &PeerEligibility) -> bool {
    snapshot.state == PeerState::Connected && peer_matches_requirements(snapshot, requirements)
}

// A peer is placeable while Connected OR still Connecting: dispatch prefers a
// warm (Connected) peer and waits for one (see `wait_for_placeable_peer`), but a
// Connecting peer remains a valid cold-dial fallback so a first submit is never
// rejected just because warm-up has not finished.
pub(super) fn peer_is_placeable(snapshot: &PeerSnapshot, requirements: &PeerEligibility) -> bool {
    matches!(snapshot.state, PeerState::Connected | PeerState::Connecting)
        && peer_matches_requirements(snapshot, requirements)
}

// A matching peer that is still warming up (Connecting) and so could yet become
// eligible. Used to bound the placement wait: once no matching peer is warming,
// waiting longer for a warm connection is pointless.
pub(super) fn peer_is_warming(snapshot: &PeerSnapshot, requirements: &PeerEligibility) -> bool {
    snapshot.state == PeerState::Connecting && peer_matches_requirements(snapshot, requirements)
}

fn peer_matches_requirements(snapshot: &PeerSnapshot, requirements: &PeerEligibility) -> bool {
    peer_matches_inventory_requirements(snapshot, requirements)
        && peer_matches_resource_requirements(snapshot, requirements)
}

fn peer_matches_inventory_requirements(
    snapshot: &PeerSnapshot,
    requirements: &PeerEligibility,
) -> bool {
    requirements
        .transport
        .as_ref()
        .is_none_or(|transport| &snapshot.transport == transport)
        && requirements
            .pool
            .as_ref()
            .is_none_or(|pool| snapshot.pools.iter().any(|value| value == pool))
        && requirements
            .tags
            .iter()
            .all(|tag| snapshot.tags.iter().any(|value| value == tag))
        && requirements.capabilities.iter().all(|capability| {
            snapshot
                .capabilities
                .iter()
                .any(|value| value == capability)
                || capability
                    .strip_prefix("node:")
                    .is_some_and(|node_id| node_id == snapshot.node_id)
        })
}

fn peer_matches_resource_requirements(
    snapshot: &PeerSnapshot,
    requirements: &PeerEligibility,
) -> bool {
    if requirements.cpu_cores.is_none() && requirements.memory_mb.is_none() {
        return true;
    }
    if snapshot.state != PeerState::Connected {
        return true;
    }
    let Some(summary) = snapshot.resource_summary.as_deref() else {
        return false;
    };
    let resources = PeerResources::parse(summary);
    requirements.cpu_cores.is_none_or(|required| {
        resources
            .cpu_total
            .is_none_or(|capacity| capacity >= required)
    }) && requirements.memory_mb.is_none_or(|required| {
        resources
            .memory_total_mb
            .is_none_or(|capacity| capacity >= required)
    })
}

pub fn first_eligible_or_error(
    peers: &[PeerSnapshot],
    requirements: &PeerEligibility,
) -> Result<PeerSnapshot> {
    peers
        .iter()
        .find(|snapshot| peer_is_eligible(snapshot, requirements))
        .cloned()
        .ok_or_else(|| anyhow!("no eligible Tor peers"))
}

pub fn first_placeable_or_error(
    peers: &[PeerSnapshot],
    requirements: &PeerEligibility,
) -> Result<PeerSnapshot> {
    if let Some(peer) = peers
        .iter()
        .find(|snapshot| peer_is_placeable(snapshot, requirements))
        .cloned()
    {
        return Ok(peer);
    }
    Err(placement_diagnostic(peers, requirements))
}

fn placement_diagnostic(peers: &[PeerSnapshot], requirements: &PeerEligibility) -> anyhow::Error {
    if peers.is_empty() {
        return anyhow!("no configured Tor peers");
    }
    if all_state(peers, PeerState::Unreachable) {
        return anyhow!("all Tor peers are unreachable");
    }
    if all_state(peers, PeerState::AuthFailed) {
        return anyhow!("all Tor peers are auth failed");
    }
    if all_state(peers, PeerState::ProtocolMismatch) {
        return anyhow!("all Tor peers have protocol mismatch");
    }
    if !peers
        .iter()
        .any(|snapshot| peer_matches_inventory_requirements(snapshot, requirements))
    {
        return anyhow!("no Tor peers match pool/tag/capability/transport requirements");
    }
    if has_resource_requirements(requirements)
        && peers.iter().any(|snapshot| {
            matches!(snapshot.state, PeerState::Connected | PeerState::Connecting)
                && peer_matches_inventory_requirements(snapshot, requirements)
        })
    {
        return anyhow!("{}", capacity_diagnostic(peers, requirements));
    }
    anyhow!("no placeable Tor peers")
}

fn all_state(peers: &[PeerSnapshot], state: PeerState) -> bool {
    peers.iter().all(|snapshot| snapshot.state == state)
}
