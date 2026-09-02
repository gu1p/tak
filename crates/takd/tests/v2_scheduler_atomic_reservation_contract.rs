use std::num::NonZeroU32;
use std::sync::{Arc, Barrier};

use tak_core::v2::{ContainerSource, RuntimeResources, TaskRuntime};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[test]
fn concurrent_reservations_never_exceed_run_or_node_capacity() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("atomic", 4);
    request.run.options.max_parallel_jobs = NonZeroU32::new(2).unwrap();
    let run = store.submit(&request, "uid:1").unwrap();
    store
        .upload_workspace(
            &run.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE,
        )
        .unwrap();
    store.commit(&run.run_id).unwrap();
    let nodes = Arc::new([
        SchedulerNode::with_execution_slots("worker-a", 1),
        SchedulerNode::with_execution_slots("worker-b", 1),
    ]);
    let barrier = Arc::new(Barrier::new(4));
    let threads = (0..4)
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
    let reserved = threads
        .into_iter()
        .filter_map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(reserved.len(), 2);
    assert_ne!(reserved[0].node_id, reserved[1].node_id);
    assert_eq!(store.pending_dispatches().unwrap().len(), 2);
}

#[test]
fn aggregate_reservations_do_not_overflow_sqlite_integer_sums() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("large-reservations", 3);
    let amount = i64::MAX as u64 / 2 + 1;
    for (job, task) in request.run.jobs.iter_mut().zip(&mut request.run.tasks) {
        job.resources.cpu_millis = amount;
        job.resources.memory_bytes = 1;
        task.runtime = Some(
            TaskRuntime::configured_container(
                ContainerSource::Image {
                    image: "alpine:3.20".into(),
                },
                vec![],
                Default::default(),
                Some(RuntimeResources {
                    cpu_millis: amount,
                    memory_bytes: 1,
                }),
            )
            .unwrap(),
        );
    }
    let run = store.submit(&request, "uid:1").unwrap();
    store
        .upload_workspace(
            &run.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE,
        )
        .unwrap();
    store.commit(&run.run_id).unwrap();
    let mut node = SchedulerNode::with_execution_slots("worker-a", 3);
    node.cpu_capacity_millis = u64::MAX;

    assert!(store.reserve_next(&[node.clone()]).unwrap().is_some());
    assert!(store.reserve_next(&[node.clone()]).unwrap().is_some());
    assert!(store.reserve_next(&[node]).unwrap().is_some());
}
