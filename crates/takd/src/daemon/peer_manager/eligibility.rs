use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::{PeerSnapshot, PeerState};

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

pub(super) fn peer_is_placeable(snapshot: &PeerSnapshot, requirements: &PeerEligibility) -> bool {
    matches!(snapshot.state, PeerState::Connected | PeerState::Connecting)
        && peer_matches_requirements(snapshot, requirements)
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
            .cpu_available
            .is_some_and(|available| available >= required)
    }) && requirements.memory_mb.is_none_or(|required| {
        resources
            .memory_available_mb
            .is_some_and(|available| available >= required)
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
        return anyhow!("no Tor peers have enough resource capacity");
    }
    anyhow!("no placeable Tor peers")
}

fn all_state(peers: &[PeerSnapshot], state: PeerState) -> bool {
    peers.iter().all(|snapshot| snapshot.state == state)
}

fn has_resource_requirements(requirements: &PeerEligibility) -> bool {
    requirements.cpu_cores.is_some() || requirements.memory_mb.is_some()
}

#[derive(Default)]
struct PeerResources {
    cpu_available: Option<f64>,
    memory_available_mb: Option<u64>,
}

impl PeerResources {
    fn parse(summary: &str) -> Self {
        let mut resources = Self::default();
        for part in summary.split_whitespace() {
            if let Some(value) = part.strip_prefix("cpu_available=") {
                resources.cpu_available = value.parse::<f64>().ok();
            }
            if let Some(value) = part.strip_prefix("memory_available_mb=") {
                resources.memory_available_mb = value.parse::<u64>().ok();
            }
        }
        resources
    }
}
