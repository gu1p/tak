use std::sync::{Arc, Barrier};

use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_queue,
    scheduler::{commit, independent_jobs},
};

#[test]
fn a_scoped_queue_is_acquired_atomically_and_released_on_completion() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = project_queue(independent_jobs("queue-a", 1), 1);
    let second = project_queue(independent_jobs("queue-b", 1), 1);
    commit(&store, &first, "alice");
    commit(&store, &second, "bob");
    let nodes = Arc::new([SchedulerNode::with_execution_slots("worker-a", 2)]);
    let barrier = Arc::new(Barrier::new(2));
    let threads = (0..2)
        .map(|_| {
            let store = store.clone();
            let nodes = Arc::clone(&nodes);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store.reserve_next(nodes.as_slice()).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let mut commands = threads
        .into_iter()
        .filter_map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 1);
    assert!(store.reserve_next(nodes.as_slice()).unwrap().is_none());

    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "1".repeat(64),
    };
    store
        .complete_attempt(&commands.pop().unwrap(), completion)
        .unwrap();
    assert!(store.reserve_next(nodes.as_slice()).unwrap().is_some());
}

#[test]
fn an_earlier_queue_waiter_precedes_a_fairness_favored_later_waiter() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let holder = project_queue(independent_jobs("fifo-holder", 1), 1);
    commit(&store, &holder, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];
    let active = store.reserve_next(&nodes).unwrap().unwrap();

    let early = project_queue(independent_jobs("fifo-early", 1), 1);
    let early_id = commit(&store, &early, "bob");
    let unrelated_id = commit(&store, &independent_jobs("fifo-unrelated", 1), "bob");
    let unrelated = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(unrelated.run_id, unrelated_id);
    let later = project_queue(independent_jobs("fifo-later", 1), 1);
    commit(&store, &later, "carol");

    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "8".repeat(64),
    };
    store.complete_attempt(&active, completion).unwrap();
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().run_id,
        early_id
    );
}

#[test]
fn a_queue_capacity_of_two_admits_exactly_two_attempts() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for (key, owner) in [("two-a", "alice"), ("two-b", "bob"), ("two-c", "carol")] {
        let request = project_queue(independent_jobs(key, 1), 2);
        commit(&store, &request, owner);
    }
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    store
        .complete_attempt(
            &first,
            AttemptCompletion::Succeeded {
                terminal_digest: "b".repeat(64),
            },
        )
        .unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
