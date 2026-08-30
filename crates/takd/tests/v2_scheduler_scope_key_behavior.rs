use tak_core::v2::{DefinitionScope, HoldMode};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::scoped_lock,
    scheduler::{commit, independent_jobs},
};

#[test]
fn structured_scope_keys_do_not_collide_on_colons() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut first = scoped_lock(
        independent_jobs("colon-a", 1),
        DefinitionScope::Project,
        Some("c"),
        HoldMode::During,
    );
    first.run.project_id = "a:b".into();
    let mut second = scoped_lock(
        independent_jobs("colon-b", 1),
        DefinitionScope::Project,
        Some("b:c"),
        HoldMode::During,
    );
    second.run.project_id = "a".into();
    commit(&store, &first, "alice");
    commit(&store, &second, "bob");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
