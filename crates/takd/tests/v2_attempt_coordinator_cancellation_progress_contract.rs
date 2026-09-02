use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{
    AttemptCoordinator, AttemptDispatch, AttemptObservation, AttemptTransport, DispatchCommand,
    RunStore, SchedulerNode,
};
use tokio::sync::Notify;

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn slow_cancellation_stays_in_flight_without_duplicates_or_starving_dispatch() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let run_id = commit(&store, &independent_jobs("slow-cancel", 1), "uid:1");
    store.reserve_next(&nodes).unwrap().unwrap();
    store.cancel(&run_id).unwrap();
    commit(&store, &independent_jobs("later-dispatch", 1), "uid:2");
    store.reserve_next(&nodes).unwrap().unwrap();
    let transport = Arc::new(ControlledCancel::default());
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());

    let first = tokio::time::timeout(Duration::from_secs(1), coordinator.drive_once())
        .await
        .expect("slow cancellation blocked dispatch")
        .unwrap();
    assert_eq!(first.dispatched, 1);
    assert_eq!(store.summary(&run_id).unwrap().unwrap().state, RunLifecycleState::Cancelling);
    coordinator.drive_once().await.unwrap();
    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);

    transport.release.notify_one();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !store.pending_cancellations().unwrap().is_empty() {
            coordinator.drive_once().await.unwrap();
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.summary(&run_id).unwrap().unwrap().state, RunLifecycleState::Cancelled);
}

#[derive(Default)]
struct ControlledCancel {
    calls: AtomicUsize,
    release: Notify,
}

impl AttemptTransport for ControlledCancel {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async { Ok(AttemptDispatch::Accepted) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.release.notified().await;
            Ok(())
        }
        .boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Running) }.boxed()
    }
}
