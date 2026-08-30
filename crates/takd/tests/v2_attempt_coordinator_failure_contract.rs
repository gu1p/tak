use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use futures::future::{BoxFuture, FutureExt};
use takd::{
    AttemptCoordinator, AttemptObservation, AttemptTransport, DispatchCommand, RunStore,
    SchedulerNode,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn a_transport_error_keeps_its_action_pending_without_starving_later_work() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let cancelling = commit(&store, &independent_jobs("defer-cancel", 1), "uid:1");
    store.reserve_next(&nodes).unwrap().unwrap();
    store.cancel(&cancelling).unwrap();
    commit(&store, &independent_jobs("continue-dispatch", 1), "uid:2");
    store.reserve_next(&nodes).unwrap().unwrap();

    let mut coordinator = AttemptCoordinator::new(store.clone(), Arc::new(FailingCancel));
    let report = coordinator.drive_once().await.unwrap();

    assert_eq!(report.deferred, 1);
    assert_eq!(report.dispatched, 1);
    assert_eq!(store.pending_cancellations().unwrap().len(), 1);
    assert!(store.pending_dispatches().unwrap().is_empty());
}

#[tokio::test]
async fn a_stalled_cancellation_is_bounded_and_does_not_starve_later_work() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let cancelling = commit(&store, &independent_jobs("stall-cancel", 1), "uid:1");
    store.reserve_next(&nodes).unwrap().unwrap();
    store.cancel(&cancelling).unwrap();
    commit(&store, &independent_jobs("still-dispatch", 1), "uid:2");
    store.reserve_next(&nodes).unwrap().unwrap();

    let mut coordinator = AttemptCoordinator::new(store.clone(), Arc::new(StalledCancel));
    let report = tokio::time::timeout(Duration::from_secs(1), coordinator.drive_once())
        .await
        .expect("stalled cancellation blocked the coordinator")
        .unwrap();

    assert_eq!(report.deferred, 1);
    assert_eq!(report.dispatched, 1);
    assert_eq!(store.pending_cancellations().unwrap().len(), 1);
}

struct FailingCancel;
struct StalledCancel;

impl AttemptTransport for FailingCancel {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { Ok(()) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { bail!("worker unreachable") }.boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Running) }.boxed()
    }
}

impl AttemptTransport for StalledCancel {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { Ok(()) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        futures::future::pending().boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Running) }.boxed()
    }
}
