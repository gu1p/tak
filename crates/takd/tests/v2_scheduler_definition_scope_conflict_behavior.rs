use tak_core::v2::{DefinitionScope, HoldMode};
use takd::RunStore;

use crate::support::v2_run::{constraints::scoped_lock, scheduler::independent_jobs};

#[test]
fn submitter_conflicts_are_owned_by_submitter_while_run_definitions_are_private() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = lock("submitter-a", DefinitionScope::Submitter, HoldMode::AtStart);
    let changed = lock("submitter-b", DefinitionScope::Submitter, HoldMode::During);
    store.submit(&first, "alice").unwrap();
    assert!(store.submit(&changed, "alice").is_err());
    assert!(store.submit(&changed, "bob").is_ok());

    let first = lock("run-a", DefinitionScope::Run, HoldMode::AtStart);
    let changed = lock("run-b", DefinitionScope::Run, HoldMode::During);
    assert!(store.submit(&first, "alice").is_ok());
    assert!(store.submit(&changed, "alice").is_ok());
}

#[test]
fn node_definition_conflicts_require_overlapping_candidates() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = lock("node-a", DefinitionScope::Node, HoldMode::AtStart);
    let mut overlap = lock("node-b", DefinitionScope::Node, HoldMode::During);
    overlap.run.jobs[0].placement_candidates[0].node_id = "worker-b".into();
    overlap.run.jobs[0].placement_candidates[1].node_id = "worker-c".into();
    store.submit(&first, "alice").unwrap();
    assert!(store.submit(&overlap, "bob").is_err());

    let mut disjoint = lock("node-c", DefinitionScope::Node, HoldMode::During);
    disjoint.run.jobs[0].placement_candidates[0].node_id = "worker-c".into();
    disjoint.run.jobs[0].placement_candidates[1].node_id = "worker-d".into();
    assert!(store.submit(&disjoint, "carol").is_ok());
}

#[test]
fn worktree_definition_conflicts_follow_the_stable_owner_key() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut first = scoped_lock(
        independent_jobs("tree-a", 1),
        DefinitionScope::Worktree,
        Some("worktree-a"),
        HoldMode::AtStart,
    );
    first.run.project_id = "project-a".into();
    let mut changed = scoped_lock(
        independent_jobs("tree-b", 1),
        DefinitionScope::Worktree,
        Some("worktree-a"),
        HoldMode::During,
    );
    changed.run.project_id = "project-b".into();
    for candidate in &mut changed.run.jobs[0].placement_candidates {
        candidate.node_id = format!("other-{}", candidate.node_id);
    }
    let mut different = scoped_lock(
        independent_jobs("tree-c", 1),
        DefinitionScope::Worktree,
        Some("worktree-b"),
        HoldMode::During,
    );
    different.run.project_id = "project-c".into();
    store.submit(&first, "alice").unwrap();
    assert!(store.submit(&changed, "bob").is_err());
    assert!(store.submit(&different, "carol").is_ok());
}

fn lock(key: &str, scope: DefinitionScope, hold: HoldMode) -> tak_core::v2::RunSubmission {
    scoped_lock(independent_jobs(key, 1), scope, None, hold)
}
