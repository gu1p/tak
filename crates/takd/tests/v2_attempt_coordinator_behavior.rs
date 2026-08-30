use std::sync::{Arc, Mutex};

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use takd::{
    AttemptCoordinator, AttemptObservation, AttemptTransport, DispatchCommand, RunStore,
    SchedulerNode,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn coordinator_replays_cancellation_before_dispatch_after_restart() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let node = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let cancel_run = commit(&store, &independent_jobs("drive-cancel", 1), "uid:1");
    store.reserve_next(&node).unwrap().unwrap();
    store.cancel(&cancel_run).unwrap();
    commit(&store, &independent_jobs("drive-dispatch", 1), "uid:2");
    let dispatch = store.reserve_next(&node).unwrap().unwrap();
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    let transport = Arc::new(RecordingTransport::default());
    let mut coordinator = AttemptCoordinator::new(restored.clone(), transport.clone());
    let report = coordinator.drive_once().await.unwrap();

    let actions = transport.actions.lock().unwrap().clone();
    assert!(actions[0].starts_with("cancel:"));
    assert_eq!(actions[1], format!("dispatch:{}", dispatch.fencing_token));
    assert_eq!(
        actions.len(),
        2,
        "new dispatch must not reconcile in the same pass"
    );
    assert_eq!(report.cancelled, 1);
    assert_eq!(report.dispatched, 1);
    assert_eq!(report.reconciled, 0);
    assert!(restored.pending_cancellations().unwrap().is_empty());
    assert!(restored.pending_dispatches().unwrap().is_empty());
}

#[derive(Default)]
struct RecordingTransport {
    actions: Mutex<Vec<String>>,
}

impl AttemptTransport for RecordingTransport {
    fn dispatch<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async move {
            self.record("dispatch", command);
            Ok(())
        }
        .boxed()
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async move {
            self.record("cancel", command);
            Ok(())
        }
        .boxed()
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async move {
            self.record("reconcile", command);
            Ok(AttemptObservation::Running)
        }
        .boxed()
    }
}

impl RecordingTransport {
    fn record(&self, action: &str, command: &DispatchCommand) {
        self.actions
            .lock()
            .unwrap()
            .push(format!("{action}:{}", command.fencing_token));
    }
}
