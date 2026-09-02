use std::collections::BTreeMap;
use std::sync::{Arc, Barrier};

use takd::ResultAcceptance;

use crate::support::{
    v2_cluster::{WorkerSpec, cluster_lock, mark_snapshot, peers},
    v2_run::scheduler::{commit, independent_jobs},
};

#[tokio::test]
async fn concurrent_reservations_and_snapshot_bursts_preserve_capacity_and_run_fairness() {
    let _cluster = cluster_lock().await;
    let temp = tempfile::tempdir().unwrap();
    let store = takd::RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let workers = [
        WorkerSpec::direct("worker-a", "127.0.0.1:1".parse().unwrap(), 4),
        WorkerSpec::direct("worker-b", "127.0.0.1:2".parse().unwrap(), 4),
    ];
    let peers = peers(&workers);
    let mut labels = BTreeMap::new();
    for (label, submitter) in [
        ("a1", "alice"),
        ("a2", "alice"),
        ("b1", "bob"),
        ("b2", "bob"),
    ] {
        let run = commit(&store, &independent_jobs(label, 3), submitter);
        labels.insert(run, label);
    }
    let barrier = Arc::new(Barrier::new(9));
    let threads = (0..8)
        .map(|_| {
            let (store, peers, barrier) = (store.clone(), peers.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                store.reserve_next(&peers.scheduler_nodes()).unwrap()
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    assert!(
        threads
            .into_iter()
            .all(|thread| thread.join().unwrap().is_some())
    );

    let reserved = store.pending_dispatches().unwrap();
    let order = reserved
        .iter()
        .map(|command| labels[&command.run_id])
        .collect::<Vec<_>>();
    let counts = reserved
        .iter()
        .fold(BTreeMap::new(), |mut counts, command| {
            *counts.entry(command.node_id.as_str()).or_insert(0) += 1;
            counts
        });
    assert_eq!(order, ["a1", "b1", "a2", "b2", "a1", "b1", "a2", "b2"]);
    assert_eq!(counts, BTreeMap::from([("worker-a", 4), ("worker-b", 4)]));
    for command in &reserved {
        assert_eq!(
            store.ack_dispatch(command).unwrap(),
            ResultAcceptance::Applied
        );
    }

    let barrier = Arc::new(Barrier::new(17));
    let threads = (0..16)
        .map(|index| {
            let (store, peers, barrier) = (store.clone(), peers.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                let reflected = if index % 2 == 0 { 3 } else { 4 };
                mark_snapshot(&peers, "worker-a", 4, reflected);
                mark_snapshot(&peers, "worker-b", 4, reflected);
                store.reserve_next(&peers.scheduler_nodes()).unwrap()
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    assert!(
        threads
            .into_iter()
            .all(|thread| thread.join().unwrap().is_none())
    );
}
