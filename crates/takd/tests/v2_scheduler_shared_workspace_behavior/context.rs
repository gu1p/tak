use takd::{AttemptCompletion, RunStore, SchedulerNode};

use super::shared_run;
use crate::support::v2_run::scheduler::commit;

#[test]
fn differing_contexts_wait_for_the_active_shared_workspace_view() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = shared_run("shared-context-switch", 2, "shared");
    request.run.jobs[1].context_manifest.paths.clear();
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];

    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    store
        .complete_attempt(
            &first,
            AttemptCompletion::Succeeded {
                terminal_digest: "7".repeat(64),
            },
        )
        .unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
