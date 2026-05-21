use std::collections::BTreeMap;

use anyhow::{Result, bail};
use tak_core::model::{ResolvedTask, TaskLabel, WorkspaceSpec};

use super::remote_models::TaskPlacement;
use super::session_cascade::ExecutionCascadeOverride;

mod builder;
mod eligibility;

use builder::{canonical_label, dependency_closure, fused_members, fused_task};
use eligibility::{uses_container_session, validate_containerized_execution};

#[derive(Clone)]
pub(crate) struct FusedCascade {
    pub(crate) root: TaskLabel,
    pub(crate) members: Vec<ResolvedTask>,
    pub(crate) task: ResolvedTask,
    pub(crate) placement: Option<TaskPlacement>,
}

struct FusedCascadeCandidate {
    root: TaskLabel,
    members: Vec<TaskLabel>,
    cascade: ExecutionCascadeOverride,
}

pub(crate) fn plan_fused_cascades(
    spec: &WorkspaceSpec,
    order: &[TaskLabel],
    cascaded_executions: &BTreeMap<TaskLabel, ExecutionCascadeOverride>,
) -> Result<BTreeMap<TaskLabel, FusedCascade>> {
    let mut roots = BTreeMap::<TaskLabel, ExecutionCascadeOverride>::new();
    for cascade in cascaded_executions.values() {
        roots
            .entry(cascade.root.clone())
            .or_insert_with(|| cascade.clone());
    }

    let mut candidates = Vec::new();
    for (root, cascade) in roots {
        if !uses_container_session(&cascade) {
            continue;
        }
        validate_containerized_execution(&root, &cascade.execution)?;

        let closure = dependency_closure(spec, &root)?;
        let members = order
            .iter()
            .filter(|label| closure.contains(*label))
            .cloned()
            .collect::<Vec<_>>();
        if members.is_empty() {
            continue;
        }
        candidates.push(FusedCascadeCandidate {
            root,
            members,
            cascade,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .members
            .len()
            .cmp(&left.members.len())
            .then_with(|| left.root.cmp(&right.root))
    });

    let mut owner_by_label = BTreeMap::<TaskLabel, (TaskLabel, String)>::new();
    let mut fused_by_entry = BTreeMap::new();
    for candidate in candidates {
        if let Some((_owner, fingerprint)) = owner_by_label.get(&candidate.root)
            && fingerprint == &candidate.cascade.fingerprint
        {
            continue;
        }

        for member in &candidate.members {
            if let Some((previous, _)) = owner_by_label.get(member) {
                bail!(
                    "container execution cascade conflict: task {} is reached by {} and {}",
                    canonical_label(member),
                    canonical_label(previous),
                    canonical_label(&candidate.root)
                );
            }
        }
        for member in &candidate.members {
            owner_by_label.insert(
                member.clone(),
                (
                    candidate.root.clone(),
                    candidate.cascade.fingerprint.clone(),
                ),
            );
        }

        let entry = candidate
            .members
            .first()
            .expect("candidate has at least one member")
            .clone();
        let task = fused_task(
            spec,
            &candidate.root,
            &candidate.members,
            &candidate.cascade,
        )?;
        let members = fused_members(spec, &candidate.members)?;
        fused_by_entry.insert(
            entry,
            FusedCascade {
                root: candidate.root,
                members,
                task,
                placement: candidate.cascade.placement.clone(),
            },
        );
    }

    Ok(fused_by_entry)
}
