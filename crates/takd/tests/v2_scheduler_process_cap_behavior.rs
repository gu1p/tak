use std::{num::NonZeroU32, process::Command};

use tak_core::v2::{LimiterDefinition, PlacementKind};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_process_cap,
    scheduler::{commit, independent_jobs},
};

#[test]
fn matching_host_processes_share_process_cap_with_leases() {
    std::fs::create_dir_all(".tmp").unwrap();
    let marker = format!("tak-process-cap-{}", uuid::Uuid::new_v4());
    let mut external = Command::new("/bin/sh")
        .args(["-c", "trap ':' EXIT; sleep 30", &marker])
        .spawn()
        .unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = project_process_cap(independent_jobs("process-match", 2));
    let LimiterDefinition::ProcessCap {
        max_processes,
        match_pattern,
        ..
    } = &mut request.run.limiter_definitions[0]
    else {
        unreachable!()
    };
    *max_processes = NonZeroU32::new(2).unwrap();
    *match_pattern = Some(marker);
    request.run.jobs[1].limiter_claims = request.run.jobs[0].limiter_claims.clone();
    for job in &mut request.run.jobs {
        job.placement_candidates.truncate(1);
        job.placement_candidates[0].node_id = "local".into();
        job.placement_candidates[0].kind = PlacementKind::Local;
        job.placement_candidates[0].transport = None;
    }
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 2)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    external.kill().unwrap();
    external.wait().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
