use tak_proto::worker_v2::{INCOMPLETE_PROCESS_OBSERVATIONS, WorkerProcessObservation};

use crate::daemon::scheduler::SchedulerNode;

const PROCESS_SLOT_MILLIS: u64 = 1_000;

pub(super) fn matching_usage_millis(node: &SchedulerNode, pattern: Option<&str>) -> u64 {
    let Some(pattern) = pattern else {
        return 0;
    };
    let current;
    let processes = if node.node_id == "local" {
        current = crate::daemon::process_observation::current();
        current.as_slice()
    } else {
        node.processes.as_slice()
    };
    if processes
        .iter()
        .any(|process| process.name == INCOMPLETE_PROCESS_OBSERVATIONS)
    {
        return u64::MAX;
    }
    let count = processes
        .iter()
        .filter(|process| matches(process, pattern))
        .count();
    u64::try_from(count)
        .unwrap_or(u64::MAX)
        .saturating_mul(PROCESS_SLOT_MILLIS)
}

fn matches(process: &WorkerProcessObservation, pattern: &str) -> bool {
    process.name.contains(pattern)
        || process
            .arguments
            .iter()
            .any(|argument| argument.contains(pattern))
}
