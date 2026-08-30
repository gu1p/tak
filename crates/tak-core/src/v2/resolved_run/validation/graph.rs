use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::super::{ResolvedRun, ResolvedRunError};

pub(super) fn task_graph(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    let nodes = run
        .tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<BTreeSet<_>>();
    let edges = run.tasks.iter().flat_map(|task| {
        task.dependencies
            .iter()
            .map(move |dependency| (dependency.clone(), task.task_id.clone()))
    });
    acyclic("task", &nodes, edges)
}

pub(super) fn job_graph(
    run: &ResolvedRun,
    jobs: &BTreeSet<String>,
) -> Result<(), ResolvedRunError> {
    let edges = run.job_edges.iter().map(|edge| {
        (
            edge.dependency_job_id.clone(),
            edge.dependent_job_id.clone(),
        )
    });
    acyclic("job", jobs, edges)
}

fn acyclic(
    kind: &str,
    nodes: &BTreeSet<String>,
    edges: impl IntoIterator<Item = (String, String)>,
) -> Result<(), ResolvedRunError> {
    let mut indegree = nodes
        .iter()
        .map(|node| (node.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    let mut seen = BTreeSet::new();
    for (dependency, dependent) in edges {
        if !nodes.contains(&dependency) || !nodes.contains(&dependent) {
            return Err(ResolvedRunError::new(format!(
                "{kind} edge references an unknown {kind}"
            )));
        }
        if !seen.insert((dependency.clone(), dependent.clone())) {
            return Err(ResolvedRunError::new(format!("duplicate {kind} edge")));
        }
        *indegree.get_mut(&dependent).expect("known node") += 1;
        outgoing.entry(dependency).or_default().push(dependent);
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(node, _)| node.clone())
        .collect::<VecDeque<_>>();
    let mut visited = 0;
    while let Some(node) = ready.pop_front() {
        visited += 1;
        for dependent in outgoing.get(&node).into_iter().flatten() {
            let count = indegree.get_mut(dependent).expect("known node");
            *count -= 1;
            if *count == 0 {
                ready.push_back(dependent.clone());
            }
        }
    }
    if visited != nodes.len() {
        return Err(ResolvedRunError::new(format!(
            "{kind} graph contains a cycle"
        )));
    }
    Ok(())
}
