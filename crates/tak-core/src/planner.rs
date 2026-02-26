//! DAG planning helpers for task execution ordering.
//!
//! The current implementation provides a topological sort with missing dependency and
//! cycle detection.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use thiserror::Error;

use crate::model::TaskLabel;

#[derive(Debug, Error)]
pub enum DagError {
    #[error("task graph references unknown dependency: {task} -> {dependency}")]
    MissingDependency {
        task: TaskLabel,
        dependency: TaskLabel,
    },
    #[error("cycle detected in task graph")]
    Cycle,
}

/// Returns a dependency-first task order using topological sorting.
///
/// The function fails when a dependency is missing or when the graph contains a cycle.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn topo_sort(
    deps_by_task: &BTreeMap<TaskLabel, Vec<TaskLabel>>,
) -> Result<Vec<TaskLabel>, DagError> {
    let mut in_degree: BTreeMap<TaskLabel, usize> = BTreeMap::new();
    let mut reverse_edges: BTreeMap<TaskLabel, BTreeSet<TaskLabel>> = BTreeMap::new();

    for (task, deps) in deps_by_task {
        in_degree.entry(task.clone()).or_insert(0);
        for dep in deps {
            if !deps_by_task.contains_key(dep) {
                return Err(DagError::MissingDependency {
                    task: task.clone(),
                    dependency: dep.clone(),
                });
            }
            reverse_edges
                .entry(dep.clone())
                .or_default()
                .insert(task.clone());
            *in_degree.entry(task.clone()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<TaskLabel> = in_degree
        .iter()
        .filter_map(|(task, degree)| (*degree == 0).then_some(task.clone()))
        .collect();
    let mut ordered = Vec::with_capacity(in_degree.len());

    while let Some(task) = queue.pop_front() {
        ordered.push(task.clone());
        if let Some(dependents) = reverse_edges.get(&task) {
            for dependent in dependents {
                if let Some(entry) = in_degree.get_mut(dependent) {
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    if ordered.len() != in_degree.len() {
        return Err(DagError::Cycle);
    }

    Ok(ordered)
}
