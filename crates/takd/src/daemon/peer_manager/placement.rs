use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{PeerEligibility, PeerManager, PeerSnapshot};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerPlacementSelection {
    #[default]
    Sequential,
    Shuffle,
}

pub struct PeerPlacementRequest<'a> {
    pub requirements: &'a PeerEligibility,
    pub selection: PeerPlacementSelection,
    pub task_run_id: &'a str,
    pub attempt: u32,
}

impl PeerManager {
    /// Waits briefly for a matching peer to become *connected* (warm) so a submit
    /// reuses an already-open connection instead of cold-dialing. Returns as soon
    /// as a warm peer exists, when no matching peer is still warming up (nothing
    /// to wait for), or once the timeout elapses — after which a still-Connecting
    /// peer remains a valid cold-dial fallback for placement.
    ///
    /// ```no_run
    /// // Reason: needs a running tokio runtime and a populated peer manager.
    /// # async fn demo(peers: &takd::PeerManager, reqs: &takd::PeerEligibility) {
    /// peers
    ///     .wait_for_placeable_peer(reqs, std::time::Duration::from_secs(5))
    ///     .await;
    /// # }
    /// ```
    pub async fn wait_for_placeable_peer(
        &self,
        requirements: &PeerEligibility,
        timeout: std::time::Duration,
    ) {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !self.eligible(requirements).is_empty()
                || !self.has_warming_peer(requirements)
                || tokio::time::Instant::now() >= deadline
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    fn has_warming_peer(&self, requirements: &PeerEligibility) -> bool {
        let state = self.lock_state();
        state
            .peers
            .values()
            .any(|entry| super::eligibility::peer_is_warming(&entry.snapshot, requirements))
    }

    pub fn select_placeable(
        &self,
        request: PeerPlacementRequest<'_>,
    ) -> anyhow::Result<PeerSnapshot> {
        let mut state = self.lock_state();
        let local_identity = state.local_identity.clone();
        let peers = state
            .peers
            .values()
            // Defensive: even if the local node somehow reached the peer set, it
            // must never be chosen as a placement target.
            .filter(|entry| {
                local_identity.as_ref().is_none_or(|identity| {
                    !identity.matches_peer(&entry.snapshot.node_id, &entry.snapshot.endpoint)
                })
            })
            .map(|entry| entry.snapshot.clone())
            .collect::<Vec<_>>();
        let selected = match request.selection {
            PeerPlacementSelection::Sequential => {
                super::first_placeable_or_error(&peers, request.requirements)?
            }
            PeerPlacementSelection::Shuffle => shuffled_placeable_peer(
                &peers,
                request.requirements,
                &state.placement_assignments,
                request.task_run_id,
                request.attempt,
            )?,
        };
        if request.selection == PeerPlacementSelection::Shuffle {
            *state
                .placement_assignments
                .entry(selected.node_id.clone())
                .or_insert(0) += 1;
        }
        Ok(selected)
    }
}

fn shuffled_placeable_peer(
    peers: &[PeerSnapshot],
    requirements: &PeerEligibility,
    assignments: &std::collections::BTreeMap<String, usize>,
    task_run_id: &str,
    attempt: u32,
) -> anyhow::Result<PeerSnapshot> {
    let mut candidates = peers
        .iter()
        .filter(|peer| super::eligibility::peer_is_placeable(peer, requirements))
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return super::first_placeable_or_error(peers, requirements);
    }
    candidates.sort_by(|left, right| {
        load_rank(left)
            .cmp(&load_rank(right))
            .then_with(|| {
                assignment_count(assignments, left).cmp(&assignment_count(assignments, right))
            })
            .then_with(|| {
                shuffle_rank(task_run_id, attempt, &left.node_id).cmp(&shuffle_rank(
                    task_run_id,
                    attempt,
                    &right.node_id,
                ))
            })
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    Ok(candidates.into_iter().next().expect("candidate exists"))
}

fn load_rank(peer: &PeerSnapshot) -> (u8, u32, u8, u32) {
    (
        option_presence_rank(peer.active_job_count),
        peer.active_job_count.unwrap_or(u32::MAX),
        option_presence_rank(peer.queue_depth),
        peer.queue_depth.unwrap_or(u32::MAX),
    )
}

fn option_presence_rank<T>(value: Option<T>) -> u8 {
    if value.is_some() { 0 } else { 1 }
}

fn assignment_count(
    assignments: &std::collections::BTreeMap<String, usize>,
    peer: &PeerSnapshot,
) -> usize {
    assignments.get(&peer.node_id).copied().unwrap_or(0)
}

fn shuffle_rank(task_run_id: &str, attempt: u32, node_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(task_run_id.as_bytes());
    hasher.update([0]);
    hasher.update(attempt.to_le_bytes());
    hasher.update([0]);
    hasher.update(node_id.as_bytes());
    hasher.finalize().into()
}
