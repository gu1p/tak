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
async fn confirmed_missing_work_uses_the_unknown_outcome_retry_rule() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let mut retryable = independent_jobs("missing-idempotent", 1);
    retryable.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let retry_run = commit(&store, &retryable, "uid:1");
    let mut unsafe_work = independent_jobs("missing-unsafe", 1);
    unsafe_work.run.jobs[0].idempotent = false;
    unsafe_work.run.tasks[0].idempotent = false;
    let unsafe_run = commit(&store, &unsafe_work, "uid:2");
    for _ in 0..2 {
        let command = store.reserve_next(&nodes).unwrap().unwrap();
        store.ack_dispatch(&command).unwrap();
    }

    let mut coordinator = AttemptCoordinator::new(store.clone(), Arc::new(MissingTransport));
    coordinator.drive_once().await.unwrap();

    assert_eq!(
        store.summary(&retry_run).unwrap().unwrap().state,
        RunLifecycleState::Queued
    );
    assert_eq!(
        store.summary(&unsafe_run).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let retried = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(retried.run_id, retry_run);
    assert_eq!(retried.authored_attempt, 2);
}

struct MissingTransport;

impl AttemptTransport for MissingTransport {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async { Ok(AttemptDispatch::Accepted) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { Ok(()) }.boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Missing) }.boxed()
    }
}
