use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};
use tak_core::model::{TaskLabel, WorkspaceSpec};

pub(crate) type ExecutionLabelMap = BTreeMap<TaskLabel, String>;

pub(crate) fn execution_labels_for_targets(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
) -> Result<ExecutionLabelMap> {
    let explicit_targets = targets.iter().cloned().collect::<BTreeSet<_>>();
    let mut lineages = BTreeMap::<TaskLabel, BTreeSet<String>>::new();
    for target in targets {
        collect_label_lineages(spec, target, target.to_string(), &mut lineages)?;
    }
    Ok(lineages
        .into_iter()
        .map(|(label, lineages)| {
            let execution_label = execution_label(&label, lineages, &explicit_targets);
            (label, execution_label)
        })
        .collect())
}

fn collect_label_lineages(
    spec: &WorkspaceSpec,
    label: &TaskLabel,
    lineage: String,
    lineages: &mut BTreeMap<TaskLabel, BTreeSet<String>>,
) -> Result<()> {
    lineages
        .entry(label.clone())
        .or_default()
        .insert(lineage.clone());
    let task = spec
        .tasks
        .get(label)
        .ok_or_else(|| anyhow!("missing task for label {label}"))?;
    for dep in &task.deps {
        collect_label_lineages(spec, dep, format!("{lineage}.{dep}"), lineages)?;
    }
    Ok(())
}

fn execution_label(
    label: &TaskLabel,
    lineages: BTreeSet<String>,
    explicit_targets: &BTreeSet<TaskLabel>,
) -> String {
    if explicit_targets.contains(label) || lineages.len() != 1 {
        return label.to_string();
    }
    lineages
        .into_iter()
        .next()
        .expect("label lineages are not empty")
}
