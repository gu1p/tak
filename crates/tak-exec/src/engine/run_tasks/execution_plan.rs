use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};
use tak_core::model::{
    RemoteSelectionSpec, ResolvedTask, SessionReuseSpec, TaskLabel, WorkspaceSpec,
};

use crate::engine::execution_labels::ExecutionLabelMap;
use crate::engine::fused_cascade::FusedCascade;
use crate::engine::remote_models::TaskPlacement;
use crate::engine::session_cascade::{ExecutionCascadeOverride, task_with_execution_override};

mod labels;
mod scheduler;
use labels::{execution_label_for, label_to_unit_index, member_execution_labels};
pub(super) use scheduler::run_execution_plan;

pub(super) struct ExecutionPlan {
    units: Vec<ScheduledUnit>,
    dependents: Vec<Vec<usize>>,
    remaining_deps: Vec<usize>,
}

struct ScheduledUnit {
    root: TaskLabel,
    labels: Vec<TaskLabel>,
    execution_label: String,
    kind: ScheduledUnitKind,
}

enum ScheduledUnitKind {
    Single {
        task: ResolvedTask,
        placement_override: Option<TaskPlacement>,
    },
    Fused {
        cascade: FusedCascade,
        member_execution_labels: ExecutionLabelMap,
    },
}

pub(super) fn build_execution_plan(
    spec: &WorkspaceSpec,
    order: &[TaskLabel],
    dep_map: &BTreeMap<TaskLabel, Vec<TaskLabel>>,
    cascaded_executions: &BTreeMap<TaskLabel, ExecutionCascadeOverride>,
    fused_cascades: &BTreeMap<TaskLabel, FusedCascade>,
    execution_labels: &ExecutionLabelMap,
) -> Result<ExecutionPlan> {
    let units = scheduled_units(
        spec,
        order,
        cascaded_executions,
        fused_cascades,
        execution_labels,
    )?;
    let label_to_unit = label_to_unit_index(&units);
    let mut dependency_sets = vec![BTreeSet::<usize>::new(); units.len()];
    for (unit_id, unit) in units.iter().enumerate() {
        for label in &unit.labels {
            for dep in dep_map.get(label).into_iter().flatten() {
                let dep_unit = *label_to_unit
                    .get(dep)
                    .ok_or_else(|| anyhow!("missing scheduled dependency for {dep}"))?;
                if dep_unit != unit_id {
                    dependency_sets[unit_id].insert(dep_unit);
                }
            }
        }
    }

    Ok(dependency_plan(units, dependency_sets))
}

fn dependency_plan(
    units: Vec<ScheduledUnit>,
    dependency_sets: Vec<BTreeSet<usize>>,
) -> ExecutionPlan {
    let mut dependents = vec![Vec::new(); units.len()];
    let remaining_deps = dependency_sets
        .iter()
        .map(BTreeSet::len)
        .collect::<Vec<_>>();
    for (unit_id, deps) in dependency_sets.iter().enumerate() {
        for dep in deps {
            dependents[*dep].push(unit_id);
        }
    }
    ExecutionPlan {
        units,
        dependents,
        remaining_deps,
    }
}

fn scheduled_units(
    spec: &WorkspaceSpec,
    order: &[TaskLabel],
    cascaded_executions: &BTreeMap<TaskLabel, ExecutionCascadeOverride>,
    fused_cascades: &BTreeMap<TaskLabel, FusedCascade>,
    execution_labels: &ExecutionLabelMap,
) -> Result<Vec<ScheduledUnit>> {
    let mut units = Vec::new();
    let mut covered_by_fused_cascade = BTreeSet::new();

    for label in order {
        if covered_by_fused_cascade.contains(label) {
            continue;
        }
        if let Some(fused) = fused_cascades.get(label) {
            let labels = fused_member_labels(fused);
            for label in &labels {
                covered_by_fused_cascade.insert(label.clone());
            }
            units.push(fused_unit(fused, labels, execution_labels));
            continue;
        }
        units.push(single_unit(
            spec,
            label,
            cascaded_executions,
            execution_labels,
        )?);
    }

    Ok(units)
}

fn fused_member_labels(fused: &FusedCascade) -> Vec<TaskLabel> {
    fused
        .members
        .iter()
        .map(|member| member.label.clone())
        .collect()
}

fn fused_unit(
    fused: &FusedCascade,
    labels: Vec<TaskLabel>,
    execution_labels: &ExecutionLabelMap,
) -> ScheduledUnit {
    ScheduledUnit {
        root: fused.root.clone(),
        labels,
        execution_label: execution_label_for(&fused.root, execution_labels),
        kind: ScheduledUnitKind::Fused {
            cascade: fused.clone(),
            member_execution_labels: member_execution_labels(&fused.members, execution_labels),
        },
    }
}

fn single_unit(
    spec: &WorkspaceSpec,
    label: &TaskLabel,
    cascaded_executions: &BTreeMap<TaskLabel, ExecutionCascadeOverride>,
    execution_labels: &ExecutionLabelMap,
) -> Result<ScheduledUnit> {
    let task = spec
        .tasks
        .get(label)
        .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;
    let cascade = cascaded_executions.get(label);
    let effective_task = task_with_execution_override(task, cascade);
    Ok(ScheduledUnit {
        root: label.clone(),
        labels: vec![label.clone()],
        execution_label: execution_label_for(label, execution_labels),
        kind: ScheduledUnitKind::Single {
            task: effective_task.unwrap_or_else(|| task.clone()),
            placement_override: cascade.and_then(|cascade| placement_override(label, cascade)),
        },
    })
}

fn placement_override(
    label: &TaskLabel,
    cascade: &ExecutionCascadeOverride,
) -> Option<TaskPlacement> {
    let placement = cascade.placement.clone()?;
    if label == &cascade.root
        || !matches!(
            placement.remote_selection,
            RemoteSelectionSpec::Shuffle | RemoteSelectionSpec::RoundRobin
        )
        || placement_uses_container_session(&placement)
    {
        return Some(placement);
    }
    None
}

fn placement_uses_container_session(placement: &TaskPlacement) -> bool {
    placement
        .session
        .as_ref()
        .is_some_and(|session| matches!(session.reuse, SessionReuseSpec::Container))
}
