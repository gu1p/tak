use std::collections::{BTreeMap, BTreeSet};

use super::{ResolvedRun, WorkspaceEntry, WorkspaceManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducedOutput<T> {
    pub producer_task_id: String,
    pub entry: WorkspaceEntry,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutput<T> {
    pub entry: WorkspaceEntry,
    pub payload: T,
    pub producers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutputs<T> {
    pub manifest: WorkspaceManifest,
    pub outputs: Vec<ResolvedOutput<T>>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{message}")]
pub struct OutputMergeError {
    message: String,
}

pub fn resolve_dependency_outputs<T: Clone>(
    run: &ResolvedRun,
    consumer_task_id: &str,
    produced: impl IntoIterator<Item = ProducedOutput<T>>,
) -> Result<ResolvedOutputs<T>, OutputMergeError> {
    let graph = Graph::new(run);
    let dependencies = graph.dependencies(consumer_task_id).ok_or_else(|| {
        OutputMergeError::new(format!("unknown output consumer `{consumer_task_id}`"))
    })?;
    resolve(&graph, dependencies, produced, consumer_task_id)
}

pub fn resolve_final_outputs<T: Clone>(
    run: &ResolvedRun,
    produced: impl IntoIterator<Item = ProducedOutput<T>>,
) -> Result<ResolvedOutputs<T>, OutputMergeError> {
    let graph = Graph::new(run);
    let mut scope = BTreeSet::new();
    for target in &run.targets {
        if !graph.tasks.contains_key(target.as_str()) {
            return Err(OutputMergeError::new(format!(
                "unknown output target `{target}`"
            )));
        }
        scope.insert(target.clone());
        scope.extend(graph.dependencies(target).expect("target was checked"));
    }
    resolve(&graph, scope, produced, "final run")
}

fn resolve<T: Clone>(
    graph: &Graph<'_>,
    scope: BTreeSet<String>,
    produced: impl IntoIterator<Item = ProducedOutput<T>>,
    consumer: &str,
) -> Result<ResolvedOutputs<T>, OutputMergeError> {
    let mut by_path = BTreeMap::<String, BTreeMap<String, ProducedOutput<T>>>::new();
    for output in produced {
        if !graph.tasks.contains_key(output.producer_task_id.as_str()) {
            return Err(OutputMergeError::new(format!(
                "unknown output producer `{}`",
                output.producer_task_id
            )));
        }
        if !scope.contains(&output.producer_task_id) {
            continue;
        }
        let path = output.entry.path.clone();
        let producer = output.producer_task_id.clone();
        let producers = by_path.entry(path.clone()).or_default();
        if let Some(previous) = producers.get(&producer) {
            if previous.entry != output.entry {
                return Err(OutputMergeError::new(format!(
                    "producer emitted conflicting output metadata for `{path}`"
                )));
            }
            continue;
        }
        producers.insert(producer, output);
    }
    let mut outputs = Vec::with_capacity(by_path.len());
    for (path, candidates) in by_path {
        let survivors = candidates
            .values()
            .filter(|candidate| {
                !candidates.values().any(|other| {
                    candidate.producer_task_id != other.producer_task_id
                        && graph.depends_on(&other.producer_task_id, &candidate.producer_task_id)
                })
            })
            .collect::<Vec<_>>();
        let first = survivors[0];
        if survivors
            .iter()
            .any(|candidate| candidate.entry != first.entry)
        {
            return Err(OutputMergeError::new(format!(
                "independent producers conflict on declared output `{path}` before `{consumer}`"
            )));
        }
        outputs.push(ResolvedOutput {
            entry: first.entry.clone(),
            payload: first.payload.clone(),
            producers: survivors
                .iter()
                .map(|candidate| candidate.producer_task_id.clone())
                .collect(),
        });
    }
    let manifest = WorkspaceManifest::new(outputs.iter().map(|output| output.entry.clone()))
        .map_err(|error| {
            OutputMergeError::new(format!(
                "declared output hierarchy conflicts before `{consumer}`: {error}"
            ))
        })?;
    Ok(ResolvedOutputs { manifest, outputs })
}

struct Graph<'a> {
    tasks: BTreeMap<&'a str, &'a Vec<String>>,
}

impl<'a> Graph<'a> {
    fn new(run: &'a ResolvedRun) -> Self {
        Self {
            tasks: run
                .tasks
                .iter()
                .map(|task| (task.task_id.as_str(), &task.dependencies))
                .collect(),
        }
    }

    fn dependencies(&self, task: &str) -> Option<BTreeSet<String>> {
        let direct = self.tasks.get(task)?;
        let mut result = BTreeSet::new();
        let mut pending = direct.to_vec();
        while let Some(current) = pending.pop() {
            if result.insert(current.clone())
                && let Some(dependencies) = self.tasks.get(current.as_str())
            {
                pending.extend(dependencies.iter().cloned());
            }
        }
        Some(result)
    }

    fn depends_on(&self, task: &str, dependency: &str) -> bool {
        self.dependencies(task)
            .is_some_and(|dependencies| dependencies.contains(dependency))
    }
}

impl OutputMergeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
