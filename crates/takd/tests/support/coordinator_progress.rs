use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use takd::{
    AttemptCoordinator, AttemptDispatch, AttemptObservation, AttemptTransport, DispatchCommand,
};
use tokio::sync::Notify;

pub struct ControlledTransport {
    pub stalled: &'static str,
    pub release: Notify,
    calls: Mutex<Vec<(&'static str, String)>>,
}

impl ControlledTransport {
    pub fn new(stalled: &'static str) -> Arc<Self> {
        Arc::new(Self {
            stalled,
            release: Notify::new(),
            calls: Mutex::new(Vec::new()),
        })
    }

    pub fn calls(&self, operation: &str, token: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(kind, called)| *kind == operation && called == token)
            .count()
    }

    async fn call(&self, operation: &'static str, command: &DispatchCommand) {
        self.calls
            .lock()
            .unwrap()
            .push((operation, command.fencing_token.clone()));
        if operation == self.stalled && command.node_id == "worker-a" {
            self.release.notified().await;
        }
    }
}

impl AttemptTransport for ControlledTransport {
    fn dispatch<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async move {
            self.call("dispatch", command).await;
            Ok(AttemptDispatch::Accepted)
        }
        .boxed()
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async move {
            self.call("reconcile", command).await;
            Ok(AttemptObservation::Running)
        }
        .boxed()
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async move {
            self.call("cancel", command).await;
            Ok(())
        }
        .boxed()
    }

    fn acknowledge_terminal<'a>(
        &'a self,
        command: &'a DispatchCommand,
        _: &'a str,
        _: bool,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            self.call("ack", command).await;
            Ok(())
        }
        .boxed()
    }
}

pub async fn tick<T: AttemptTransport + 'static>(coordinator: &mut AttemptCoordinator<T>) {
    tokio::time::timeout(Duration::from_secs(1), coordinator.drive_once())
        .await
        .expect("network operation blocked the driver tick")
        .unwrap();
}
