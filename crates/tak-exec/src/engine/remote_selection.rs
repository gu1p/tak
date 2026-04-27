use sha2::{Digest, Sha256};
use tak_core::model::RemoteSelectionSpec;

use super::remote_models::StrictRemoteTarget;

pub(crate) fn ordered_remote_targets_for_attempt(
    targets: &[StrictRemoteTarget],
    selection: RemoteSelectionSpec,
    task_label: &str,
    task_run_id: &str,
    attempt: u32,
) -> Vec<StrictRemoteTarget> {
    match selection {
        RemoteSelectionSpec::Sequential => targets.to_vec(),
        RemoteSelectionSpec::Shuffle => shuffled_targets(targets, task_label, task_run_id, attempt),
    }
}

fn shuffled_targets(
    targets: &[StrictRemoteTarget],
    task_label: &str,
    task_run_id: &str,
    attempt: u32,
) -> Vec<StrictRemoteTarget> {
    let mut ranked = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            (
                shuffle_rank(task_label, task_run_id, attempt, &target.node_id),
                index,
                target.clone(),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    ranked.into_iter().map(|(_, _, target)| target).collect()
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
