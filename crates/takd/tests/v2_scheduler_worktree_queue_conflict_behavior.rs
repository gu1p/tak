use std::num::NonZeroU32;

use tak_core::v2::{DefinitionScope, QueueDefinition, QueueDiscipline, RunSubmission};
use takd::RunStore;

use crate::support::v2_run::scheduler::independent_jobs;

#[test]
fn worktree_queue_conflicts_are_global_to_the_stable_owner_key() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = queue("queue-a", "worktree-a", "project-a", 1, "worker");
    let changed = queue("queue-b", "worktree-a", "project-b", 2, "other");
    let different = queue("queue-c", "worktree-b", "project-c", 2, "worker");
    store.submit(&first, "alice").unwrap();
    assert!(store.submit(&changed, "bob").is_err());
    assert!(store.submit(&different, "carol").is_ok());
}

fn queue(key: &str, owner: &str, project: &str, slots: u32, node: &str) -> RunSubmission {
    let mut request = independent_jobs(key, 1);
    request.run.project_id = project.into();
    request.run.jobs[0].placement_candidates[0].node_id = format!("{node}-a");
    request.run.jobs[0].placement_candidates[1].node_id = format!("{node}-b");
    request.run.jobs[0].queue = Some("build".into());
    request.run.queue_definitions = vec![QueueDefinition {
        name: "build".into(),
        scope: DefinitionScope::Worktree,
        scope_key: Some(owner.into()),
        max_parallel_tasks: NonZeroU32::new(slots).unwrap(),
        discipline: QueueDiscipline::Fifo,
    }];
    request
}
