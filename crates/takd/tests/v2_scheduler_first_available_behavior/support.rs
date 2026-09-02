use std::collections::BTreeMap;

use tak_core::v2::{PlacementCandidate, PlacementKind, RemoteSelection};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

pub(super) fn committed(
    key: &str,
    count: usize,
    selection: RemoteSelection,
) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, count);
    for job in &mut request.run.jobs {
        job.placement_policy.selection = selection;
        job.placement_candidates
            .push(candidate("local", PlacementKind::Local, 1));
        for candidate in &mut job.placement_candidates[..2] {
            candidate.tier = 0;
        }
    }
    let run_id = commit(&store, &request, "uid:1");
    (temp, store, run_id)
}

fn candidate(node_id: &str, kind: PlacementKind, tier: u32) -> PlacementCandidate {
    PlacementCandidate {
        node_id: node_id.into(),
        kind,
        transport: None,
        reason: "eligible".into(),
        tier,
        requirements: None,
    }
}

pub(super) fn node(node_id: &str, slots: u32) -> SchedulerNode {
    SchedulerNode::with_execution_slots(node_id, slots)
}

pub(super) fn counts(store: &RunStore, run_id: &str) -> BTreeMap<String, i32> {
    store
        .get_run(run_id)
        .unwrap()
        .unwrap()
        .jobs
        .into_iter()
        .fold(BTreeMap::new(), |mut counts, job| {
            *counts.entry(job.node_id.unwrap()).or_insert(0) += 1;
            counts
        })
}
