use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use tak_core::model::RemoteSelectionSpec;

use super::remote_models::StrictRemoteTarget;

#[derive(Debug, Default)]
pub(crate) struct RemoteSelectionState {
    assignments: BTreeMap<String, usize>,
}

impl RemoteSelectionState {
    pub(crate) fn record_assignment(&mut self, node_id: &str) {
        *self.assignments.entry(node_id.to_string()).or_insert(0) += 1;
    }

    pub(crate) fn release_assignment(&mut self, node_id: &str) {
        let Some(count) = self.assignments.get_mut(node_id) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.assignments.remove(node_id);
        }
    }

    pub(crate) fn replace_assignment(&mut self, previous: &str, next: &str) {
        if previous == next {
            return;
        }
        self.release_assignment(previous);
        self.record_assignment(next);
    }

    pub(crate) fn assignment_count(&self, node_id: &str) -> usize {
        self.assignments.get(node_id).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SharedRemoteSelectionState {
    inner: Arc<Mutex<RemoteSelectionState>>,
}

impl SharedRemoteSelectionState {
    pub(crate) fn reserve_ordered_targets_for_attempt(
        &self,
        targets: &[StrictRemoteTarget],
        selection: RemoteSelectionSpec,
        task_label: &str,
        task_run_id: &str,
        attempt: u32,
    ) -> Vec<StrictRemoteTarget> {
        let mut guard = self.inner.lock().expect("remote selection state lock");
        let ordered = ordered_remote_targets_for_attempt(
            targets,
            selection,
            task_label,
            task_run_id,
            attempt,
            &guard,
        );
        if matches!(selection, RemoteSelectionSpec::Shuffle)
            && let Some(target) = ordered.first()
        {
            guard.record_assignment(&target.node_id);
        }
        ordered
    }

    pub(crate) fn confirm_selected_target(
        &self,
        selection: RemoteSelectionSpec,
        reserved_node_id: Option<&str>,
        selected_node_id: &str,
    ) {
        if !matches!(selection, RemoteSelectionSpec::Shuffle) {
            return;
        }
        let Some(reserved_node_id) = reserved_node_id else {
            return;
        };
        self.inner
            .lock()
            .expect("remote selection state lock")
            .replace_assignment(reserved_node_id, selected_node_id);
    }

    pub(crate) fn release_reserved_target(
        &self,
        selection: RemoteSelectionSpec,
        reserved_node_id: Option<&str>,
    ) {
        if !matches!(selection, RemoteSelectionSpec::Shuffle) {
            return;
        }
        let Some(reserved_node_id) = reserved_node_id else {
            return;
        };
        self.inner
            .lock()
            .expect("remote selection state lock")
            .release_assignment(reserved_node_id);
    }

    pub(crate) fn replace_assignment(
        &self,
        selection: RemoteSelectionSpec,
        previous_node_id: &str,
        next_node_id: &str,
    ) {
        if !matches!(selection, RemoteSelectionSpec::Shuffle) {
            return;
        }
        self.inner
            .lock()
            .expect("remote selection state lock")
            .replace_assignment(previous_node_id, next_node_id);
    }
}

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
        RemoteSelectionSpec::Shuffle => {
            shuffled_targets(targets, task_label, task_run_id, attempt, state)
        }
    }
}

fn shuffled_targets(
    targets: &[StrictRemoteTarget],
    task_label: &str,
    task_run_id: &str,
    attempt: u32,
    state: &RemoteSelectionState,
) -> Vec<StrictRemoteTarget> {
    let mut ranked = targets
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
    ranked.into_iter().map(|(_, _, _, target)| target).collect()
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
