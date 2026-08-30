use tak_core::v2::{DefinitionScope, HoldMode, RunSubmission};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::scoped_lock,
    scheduler::{commit, independent_jobs},
};

#[test]
fn worktree_capacity_is_shared_only_by_the_same_stable_key() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = worktree_run("tree-first", "worktree-a", "project-a");
    let same = worktree_run("tree-same", "worktree-a", "project-b");
    let different = worktree_run("tree-different", "worktree-b", "project-c");
    let first_id = commit(&store, &first, "alice");
    commit(&store, &same, "bob");
    let different_id = commit(&store, &different, "carol");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];

    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().run_id,
        first_id
    );
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().run_id,
        different_id
    );
    assert!(store.reserve_next(&nodes).unwrap().is_none());
}

fn worktree_run(key: &str, owner: &str, project: &str) -> RunSubmission {
    let mut request = scoped_lock(
        independent_jobs(key, 1),
        DefinitionScope::Worktree,
        Some(owner),
        HoldMode::During,
    );
    request.run.project_id = project.into();
    request
}
