use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail};
use tak_core::model::{TaskLabel, WorkspaceSpec};

use super::types_and_assets::{GraphEdge, GraphNode, GraphPayload};

pub(super) fn build_graph_payload(
    spec: &WorkspaceSpec,
    target: Option<&TaskLabel>,
) -> Result<GraphPayload> {
    let selected = selected_labels(spec, target)?;
    let target_label = target.map(ToString::to_string);

    let mut edges = Vec::<GraphEdge>::new();
    let mut dependents = BTreeMap::<String, usize>::new();

    for label in &selected {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow!("missing task for label {label}"))?;
        for dep in &task.deps {
            if selected.contains(dep) {
                edges.push(GraphEdge {
                    from: dep.to_string(),
                    to: label.to_string(),
                });
                *dependents.entry(dep.to_string()).or_insert(0) += 1;
            }
        }
    }

    edges.sort_by(|left, right| left.from.cmp(&right.from).then(left.to.cmp(&right.to)));

    let mut nodes = Vec::<GraphNode>::new();
    for label in &selected {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow!("missing task for label {label}"))?;

        nodes.push(GraphNode {
            id: label.to_string(),
            label: label.to_string(),
            package: label.package.clone(),
            task: label.name.clone(),
            deps: task
                .deps
                .iter()
                .filter(|dep| selected.contains(*dep))
                .count(),
            dependents: dependents.get(&label.to_string()).copied().unwrap_or(0),
        });
    }

    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(GraphPayload {
        target: target_label,
        nodes,
        edges,
    })
}

fn selected_labels(
    spec: &WorkspaceSpec,
    target: Option<&TaskLabel>,
) -> Result<BTreeSet<TaskLabel>> {
    let Some(target) = target else {
        return Ok(spec.tasks.keys().cloned().collect());
    };

    if !spec.tasks.contains_key(target) {
        bail!("task not found: {target}");
    }

    let mut selected = BTreeSet::<TaskLabel>::new();
    let mut stack = vec![target.clone()];

    while let Some(current) = stack.pop() {
        if !selected.insert(current.clone()) {
            continue;
        }

        let task = spec
            .tasks
            .get(&current)
            .ok_or_else(|| anyhow!("task not found while walking closure: {current}"))?;
        for dep in &task.deps {
            stack.push(dep.clone());
        }
    }

    Ok(selected)
}
