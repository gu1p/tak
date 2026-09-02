use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

const MAX_PROCESSES: usize = 4_096;
const MAX_NAME_BYTES: usize = 1_024;
const MAX_ARGUMENTS: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 16 * 1_024;

pub const INCOMPLETE_PROCESS_OBSERVATIONS: &str = "process-observations-incomplete";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerProcessObservation {
    pub name: String,
    pub arguments: Vec<String>,
}

/// Bounds host-process observations for one encoded worker snapshot.
///
/// Invalid or over-budget observations are replaced by a reserved incomplete marker so a
/// scheduler can conservatively saturate process caps.
///
/// ```rust
/// use tak_proto::worker_v2::{
///     INCOMPLETE_PROCESS_OBSERVATIONS, WorkerProcessObservation,
///     bounded_process_observations,
/// };
///
/// let observations = vec![WorkerProcessObservation {
///     name: "tool".into(),
///     arguments: vec!["x".repeat(20_000)],
/// }];
/// let bounded = bounded_process_observations(observations, 1_024);
/// assert_eq!(bounded[0].name, INCOMPLETE_PROCESS_OBSERVATIONS);
/// ```
#[must_use]
pub fn bounded_process_observations(
    observations: Vec<WorkerProcessObservation>,
    max_encoded_bytes: usize,
) -> Vec<WorkerProcessObservation> {
    let marker = incomplete_marker();
    let marker_size = encoded_size(&marker).unwrap_or(usize::MAX);
    let mut bounded = Vec::new();
    let mut encoded_bytes = 2_usize;
    let mut incomplete = false;
    for observation in observations {
        if !valid_observation(&observation) {
            incomplete = true;
            continue;
        }
        let Some(size) = encoded_size(&observation) else {
            incomplete = true;
            continue;
        };
        let delimiter = usize::from(!bounded.is_empty());
        let with_observation = encoded_bytes.saturating_add(delimiter).saturating_add(size);
        let with_marker = with_observation
            .saturating_add(1)
            .saturating_add(marker_size);
        if bounded.len() < MAX_PROCESSES.saturating_sub(1) && with_marker <= max_encoded_bytes {
            bounded.push(observation);
            encoded_bytes = with_observation;
        } else {
            incomplete = true;
        }
    }
    if incomplete {
        bounded.push(marker);
    }
    bounded
}

pub(super) fn validate(processes: &[WorkerProcessObservation]) -> Result<()> {
    if processes.len() > MAX_PROCESSES {
        bail!("worker process observation count exceeds the protocol limit");
    }
    for process in processes {
        if !valid_observation(process) {
            bail!("worker process observation is invalid");
        }
    }
    Ok(())
}

fn incomplete_marker() -> WorkerProcessObservation {
    WorkerProcessObservation {
        name: INCOMPLETE_PROCESS_OBSERVATIONS.into(),
        arguments: Vec::new(),
    }
}

fn valid_observation(process: &WorkerProcessObservation) -> bool {
    valid_text(&process.name, MAX_NAME_BYTES)
        && process.arguments.len() <= MAX_ARGUMENTS
        && process
            .arguments
            .iter()
            .all(|argument| valid_text(argument, MAX_ARGUMENT_BYTES))
}

fn encoded_size(process: &WorkerProcessObservation) -> Option<usize> {
    serde_json::to_vec(process)
        .ok()
        .map(|encoded| encoded.len())
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    value.len() <= max_bytes && !value.chars().any(char::is_control)
}
