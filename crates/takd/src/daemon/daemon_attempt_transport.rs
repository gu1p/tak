use anyhow::{Result, bail};
use futures::future::BoxFuture;

use super::attempt_coordinator::{AttemptDispatch, AttemptObservation, AttemptTransport};
use super::scheduler::DispatchCommand;
use super::{LocalAttemptTransport, RemoteAttemptTransport};

pub(crate) struct DaemonAttemptTransport {
    local: LocalAttemptTransport,
    remote: RemoteAttemptTransport,
}

impl DaemonAttemptTransport {
    pub(crate) fn new(local: LocalAttemptTransport, remote: RemoteAttemptTransport) -> Self {
        Self { local, remote }
    }

    fn remote(command: &DispatchCommand) -> Result<bool> {
        match (command.node_id.as_str(), command.transport.as_deref()) {
            ("local", None) => Ok(false),
            ("local", Some(_)) => bail!("local dispatch must not have a remote transport"),
            (_, Some("direct" | "tor")) => Ok(true),
            (_, Some(other)) => bail!("unsupported remote transport `{other}`"),
            (_, None) => bail!("remote dispatch is missing its persisted transport"),
        }
    }
}

impl AttemptTransport for DaemonAttemptTransport {
    fn dispatch<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptDispatch>> {
        match Self::remote(command) {
            Ok(true) => self.remote.dispatch(command),
            Ok(false) => self.local.dispatch(command),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        match Self::remote(command) {
            Ok(true) => self.remote.cancel_and_wait(command),
            Ok(false) => self.local.cancel_and_wait(command),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        match Self::remote(command) {
            Ok(true) => self.remote.reconcile(command),
            Ok(false) => self.local.reconcile(command),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }

    fn acknowledge_terminal<'a>(
        &'a self,
        command: &'a DispatchCommand,
        terminal_digest: &'a str,
        run_terminal: bool,
    ) -> BoxFuture<'a, Result<()>> {
        match Self::remote(command) {
            Ok(true) => self
                .remote
                .acknowledge_terminal(command, terminal_digest, run_terminal),
            Ok(false) => self
                .local
                .acknowledge_terminal(command, terminal_digest, run_terminal),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }
}
