use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::future::{BoxFuture, FutureExt};
use takd::{
    AttemptCoordinator, AttemptDispatch, AttemptObservation, AttemptTransport, DispatchCommand,
    NodeLossResolution, RunStore, SchedulerNode,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn a_lost_dispatch_response_never_retries_non_idempotent_work() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("dispatch-response-loss", 2);
    request.run.options.keep_going = true;
    for job in &mut request.run.jobs {
        job.retry.max_attempts = NonZeroU32::new(2).unwrap();
    }
    request.run.tasks[1].idempotent = false;
    request.run.jobs[1].idempotent = false;
    let run_id = commit(&store, &request, "alice");
    let lost = [SchedulerNode::with_execution_slots("worker-a", 2)];
    for _ in 0..2 {
        store.reserve_next(&lost).unwrap().unwrap();
    }

    let mut coordinator = AttemptCoordinator::new(store.clone(), Arc::new(DroppedResponse));
    let report = coordinator.drive_once().await.unwrap();
    assert_eq!(report.deferred, 2);
    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );

    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.jobs[0].state, "retrying");
    assert_eq!(details.jobs[1].state, "failed");
    let retry = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-b", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(retry.job_id, "job-0");
    assert_eq!(retry.authored_attempt, 2);
}

struct DroppedResponse;

impl AttemptTransport for DroppedResponse {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async { Err(anyhow!("worker accepted, response lost")) }.boxed()
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
