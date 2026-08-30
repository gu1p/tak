use tak_proto::local_daemon::v2::RunEventKind;
use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn current_fence_enters_a_persisted_output_committing_state_once() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let run_id = commit(&store, &independent_jobs("output-committing", 1), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(
        store.begin_output_commit(&command).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        store.begin_output_commit(&command).unwrap(),
        ResultAcceptance::Duplicate
    );
    let job = store.get_run(&run_id).unwrap().unwrap().jobs.remove(0);
    assert_eq!(job.state, "output_committing");
    let events = store.events_after(&run_id, 0).unwrap();
    let lifecycle = events
        .iter()
        .filter_map(|event| {
            matches!(
                event.kind,
                RunEventKind::Running | RunEventKind::OutputCommitting
            )
            .then_some(event.kind)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lifecycle,
        [RunEventKind::Running, RunEventKind::OutputCommitting]
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == RunEventKind::OutputCommitting)
            .count(),
        1
    );
}
