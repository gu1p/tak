use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{
    AttemptCoordinator, AttemptDispatch, AttemptObservation, AttemptTransport, DispatchCommand,
    RunStore, SchedulerNode,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn confirmed_stale_dispatch_uses_unknown_outcome_rules_instead_of_retrying_the_outbox() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("stale-dispatch", 1);
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    store.reserve_next(&nodes).unwrap().unwrap();

    let mut coordinator = AttemptCoordinator::new(store.clone(), Arc::new(StaleDispatch));
    let report = coordinator.drive_once().await.unwrap();

    assert_eq!(report.reconciled, 1);
    assert!(store.pending_dispatches().unwrap().is_empty());
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Queued
    );
    assert_eq!(store.reserve_next(&nodes).unwrap().unwrap().authored_attempt, 2);
}

struct StaleDispatch;

impl AttemptTransport for StaleDispatch {
    fn dispatch<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async { Ok(AttemptDispatch::Stale) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { Ok(()) }.boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Running) }.boxed()
    }
}
