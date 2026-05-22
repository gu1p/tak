use std::collections::BTreeMap;

use tak_core::model::{ResolvedTask, TaskLabel};

use super::ScheduledUnit;
use crate::engine::execution_labels::ExecutionLabelMap;

pub(super) fn execution_label_for(
    label: &TaskLabel,
    execution_labels: &ExecutionLabelMap,
) -> String {
    execution_labels
        .get(label)
        .cloned()
        .unwrap_or_else(|| label.to_string())
}

pub(super) fn member_execution_labels(
    members: &[ResolvedTask],
    execution_labels: &ExecutionLabelMap,
) -> ExecutionLabelMap {
    members
        .iter()
        .map(|member| {
            (
                member.label.clone(),
                execution_label_for(&member.label, execution_labels),
            )
        })
        .collect()
}

pub(super) fn label_to_unit_index(units: &[ScheduledUnit]) -> BTreeMap<TaskLabel, usize> {
    let mut labels = BTreeMap::new();
    for (unit_id, unit) in units.iter().enumerate() {
        for label in &unit.labels {
            labels.insert(label.clone(), unit_id);
        }
    }
    labels
}
