use anyhow::{Context, Result, bail};
use tak_proto::local_daemon::v2::{Response, RunLifecycleState};

#[derive(Clone, Copy)]
pub(super) enum Action {
    RequestCancellation,
    Detach,
}

pub(super) enum CancellationOutcome {
    Persisted,
    AlreadyTerminal,
}

pub(super) struct State {
    cancellation_requested: bool,
    signals: tokio::signal::unix::Signal,
}

impl State {
    pub(super) fn new() -> Result<Self> {
        let signals = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .context("register run attachment interrupt")?;
        Ok(Self {
            cancellation_requested: false,
            signals,
        })
    }

    pub(super) async fn next(&mut self) -> Result<Action> {
        self.signals
            .recv()
            .await
            .context("wait for run attachment interrupt")?;
        Ok(self.record())
    }

    pub(super) fn record(&mut self) -> Action {
        if self.cancellation_requested {
            return Action::Detach;
        }
        self.cancellation_requested = true;
        Action::RequestCancellation
    }
}

pub(super) fn validate_cancellation(
    run_id: &str,
    response: &Response,
) -> Result<CancellationOutcome> {
    let Response::CancellationAccepted {
        run_id: response_run,
        state,
        ..
    } = response
    else {
        bail!("local takd returned an unexpected CancelRun response")
    };
    if response_run != run_id {
        bail!("local takd returned a mismatched CancelRun response");
    }
    match state {
        RunLifecycleState::Cancelling | RunLifecycleState::Cancelled => {
            Ok(CancellationOutcome::Persisted)
        }
        RunLifecycleState::Succeeded | RunLifecycleState::Failed => {
            Ok(CancellationOutcome::AlreadyTerminal)
        }
        _ => bail!("local takd returned an invalid CancelRun state"),
    }
}
