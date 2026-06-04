use sha2::{Digest, Sha256};
use tak_core::model::RemoteSelectionSpec;

use crate::engine::remote_models::StrictRemoteTarget;
use crate::engine::remote_selection::RemoteSelectionState;

pub(crate) fn ordered_remote_targets_for_attempt(
    targets: &[StrictRemoteTarget],
    selection: RemoteSelectionSpec,
    task_label: &str,
    task_run_id: &str,
    attempt: u32,
    state: &RemoteSelectionState,
) -> Vec<StrictRemoteTarget> {
    match selection {
        RemoteSelectionSpec::Sequential => targets.to_vec(),
        RemoteSelectionSpec::RoundRobin => round_robin_targets(targets, state),
        RemoteSelectionSpec::Shuffle => {
            shuffled_targets(targets, task_label, task_run_id, attempt, state)
        }
    }
}

pub(super) fn concrete_target_count(targets: &[StrictRemoteTarget]) -> usize {
    targets
        .iter()
        .filter(|target| !target.is_daemon_tor_placement())
        .count()
}

pub(super) fn round_robin_key(targets: &[StrictRemoteTarget]) -> Vec<String> {
    let mut key = targets
        .iter()
        .filter(|target| !target.is_daemon_tor_placement())
        .map(|target| target.node_id.clone())
        .collect::<Vec<_>>();
    key.sort_unstable();
    key
}

fn round_robin_targets(
    targets: &[StrictRemoteTarget],
    state: &RemoteSelectionState,
) -> Vec<StrictRemoteTarget> {
    let (concrete_targets, daemon_fallbacks) = split_daemon_fallbacks(targets);
    if concrete_targets.is_empty() {
        return daemon_fallbacks;
    }
    let start = state.round_robin_cursor(&concrete_targets) % concrete_targets.len();
    let mut ordered = concrete_targets[start..].to_vec();
    ordered.extend_from_slice(&concrete_targets[..start]);
    ordered.extend(daemon_fallbacks);
    ordered
}

fn shuffled_targets(
    targets: &[StrictRemoteTarget],
    task_label: &str,
    task_run_id: &str,
    attempt: u32,
    state: &RemoteSelectionState,
) -> Vec<StrictRemoteTarget> {
    let (concrete_targets, daemon_fallbacks) = split_daemon_fallbacks(targets);
    let mut ranked = concrete_targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            (
                state.assignment_count(&target.node_id),
                shuffle_rank(task_label, task_run_id, attempt, &target.node_id),
                index,
                target.clone(),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    let mut ordered = ranked
        .into_iter()
        .map(|(_, _, _, target)| target)
        .collect::<Vec<_>>();
    ordered.extend(daemon_fallbacks);
    ordered
}

fn split_daemon_fallbacks(
    targets: &[StrictRemoteTarget],
) -> (Vec<StrictRemoteTarget>, Vec<StrictRemoteTarget>) {
    let mut concrete_targets = Vec::new();
    let mut daemon_fallbacks = Vec::new();
    for target in targets {
        if target.is_daemon_tor_placement() {
            daemon_fallbacks.push(target.clone());
        } else {
            concrete_targets.push(target.clone());
        }
    }
    (concrete_targets, daemon_fallbacks)
}

fn shuffle_rank(task_label: &str, task_run_id: &str, attempt: u32, node_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(task_label.as_bytes());
    hasher.update([0]);
    hasher.update(task_run_id.as_bytes());
    hasher.update([0]);
    hasher.update(attempt.to_le_bytes());
    hasher.update([0]);
    hasher.update(node_id.as_bytes());
    hasher.finalize().into()
}
