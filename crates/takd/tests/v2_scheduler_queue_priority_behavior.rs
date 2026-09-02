use std::num::NonZeroU32;

use tak_core::v2::{QueueDiscipline, RunSubmission};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_queue,
    scheduler::{commit, independent_jobs},
};

#[test]
fn requested_queue_slots_are_reserved_atomically() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = queued_jobs("slots", 2, 3, QueueDiscipline::Fifo, &[2, 2], &[0, 0]);
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];

    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    store
        .complete_attempt(
            &first,
            AttemptCompletion::Succeeded {
                terminal_digest: "1".repeat(64),
            },
        )
        .unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}

#[test]
fn priority_queue_chooses_highest_priority_ready_job_then_oldest() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = queued_jobs(
        "priority",
        3,
        1,
        QueueDiscipline::Priority,
        &[1, 1, 1],
        &[10, 100, 100],
    );
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];

    assert_eq!(store.reserve_next(&nodes).unwrap().unwrap().job_id, "job-1");
}

fn queued_jobs(
    key: &str,
    count: usize,
    capacity: u32,
    discipline: QueueDiscipline,
    slots: &[u32],
    priorities: &[i32],
) -> RunSubmission {
    let mut request = project_queue(independent_jobs(key, count), capacity);
    request.run.queue_definitions[0].discipline = discipline;
    for (index, job) in request.run.jobs.iter_mut().enumerate() {
        job.queue = Some("shared".into());
        job.queue_slots = NonZeroU32::new(slots[index]).unwrap();
        job.queue_priority = priorities[index];
    }
    request
}
