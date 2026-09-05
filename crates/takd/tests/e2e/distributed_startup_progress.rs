use std::num::NonZeroU32;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::local_daemon::v2::RunEventKind;
use takd::TorBroker;

use crate::support::{
    gated_worker::GatedWorker,
    v2_cluster::{Origin, WorkerSpec, cluster_lock, peers},
    v2_run::scheduler::{commit, independent_jobs},
    worker_http::start_server_for_node,
};

#[tokio::test]
async fn two_worker_run_finishes_fast_work_while_the_other_transfer_is_stalled() {
    let _cluster = cluster_lock().await;
    let slow = start_server_for_node("worker-a").await;
    let fast = start_server_for_node("worker-b").await;
    let gate = GatedWorker::start(slow.addr).await;
    let peers = peers(&[
        WorkerSpec::direct("worker-a", gate.addr, 10),
        WorkerSpec::direct("worker-b", fast.addr, 10),
    ]);
    let origin = Origin::start(peers, TorBroker::new()).await;
    let mut submission = independent_jobs("distributed-startup", 2);
    submission.run.options.max_parallel_jobs = NonZeroU32::new(10).unwrap();
    for task in &mut submission.run.tasks {
        task.steps = vec![Step::Cmd {
            argv: vec!["/bin/sh".into(), "-c".into(), "printf 'started\\n'".into()],
            cwd: None,
            env: Default::default(),
        }];
    }
    let run = commit(&origin.store, &submission, "alice");
    tokio::time::timeout(Duration::from_secs(5), gate.started.notified())
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let jobs = origin.store.get_run(&run).unwrap().unwrap().jobs;
            if jobs[1].state == "succeeded" {
                assert_eq!(jobs[0].state, "transferring");
                assert_eq!(jobs[1].node_id.as_deref(), Some("worker-b"));
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("slow worker blocked execution or completion on fast worker");
    assert!(
        origin
            .store
            .events_after(&run, 0)
            .unwrap()
            .iter()
            .any(|event| {
                event.job_id.as_deref() == Some("job-1") && event.kind == RunEventKind::Stdout
            })
    );
    gate.release();
    origin.wait_for_terminal(&run).await;
    assert!(
        origin
            .store
            .get_run(&run)
            .unwrap()
            .unwrap()
            .jobs
            .iter()
            .all(|job| job.state == "succeeded")
    );
}
