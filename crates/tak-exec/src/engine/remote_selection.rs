use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use tak_core::model::RemoteSelectionSpec;

use super::remote_models::StrictRemoteTarget;
use super::workspace_upload_cache::SharedWorkspaceUploadCache;

mod ordering;
pub(crate) use ordering::ordered_remote_targets_for_attempt;

#[derive(Debug, Default)]
pub(crate) struct RemoteSelectionState {
    assignments: BTreeMap<String, usize>,
    round_robin_cursors: BTreeMap<Vec<String>, usize>,
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

    fn round_robin_cursor(&self, targets: &[StrictRemoteTarget]) -> usize {
        let key = ordering::round_robin_key(targets);
        self.round_robin_cursors.get(&key).copied().unwrap_or(0)
    }

    fn advance_round_robin(&mut self, targets: &[StrictRemoteTarget]) {
        let concrete_count = ordering::concrete_target_count(targets);
        if concrete_count == 0 {
            return;
        }
        let cursor = self
            .round_robin_cursors
            .entry(ordering::round_robin_key(targets))
            .or_insert(0);
        *cursor = cursor.saturating_add(1) % concrete_count;
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SharedRemoteSelectionState {
    inner: Arc<Mutex<RemoteSelectionState>>,
    /// Per-run cache of workspace uploads, so the same repository is uploaded to a node only
    /// once per job. It carries its own internal synchronization (never the `inner` mutex
    /// above), so it can be awaited on without holding the selection lock.
    upload_cache: SharedWorkspaceUploadCache,
}

impl SharedRemoteSelectionState {
    /// The per-run workspace-upload cache (see [`SharedWorkspaceUploadCache`]).
    ///
    /// ```no_run
    /// # // Reason: This private accessor is exercised through remote execution tests.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn upload_cache(&self) -> &SharedWorkspaceUploadCache {
        &self.upload_cache
    }

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
        if matches!(selection, RemoteSelectionSpec::RoundRobin) {
            guard.advance_round_robin(targets);
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
