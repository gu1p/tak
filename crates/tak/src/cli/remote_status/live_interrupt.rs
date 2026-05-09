use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::sleep;

pub(super) async fn interruptible<I, F, T>(
    interrupt: &mut I,
    operation: F,
) -> Result<PollOutcome<T>>
where
    I: InterruptSource + ?Sized,
    F: Future<Output = Result<T>>,
{
    tokio::select! {
        output = operation => output.map(PollOutcome::Completed),
        signal = interrupt.interrupted() => {
            signal.context("wait for remote status interrupt")?;
            Ok(PollOutcome::Interrupted)
        }
    }
}

pub(super) enum PollOutcome<T> {
    Completed(T),
    Interrupted,
}

pub(super) trait InterruptSource {
    fn interrupted(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>;
}

#[cfg(unix)]
pub(super) struct InterruptListener {
    signal: tokio::signal::unix::Signal,
}

#[cfg(windows)]
pub(super) struct InterruptListener {
    signal: tokio::signal::windows::CtrlC,
}

#[cfg(not(any(unix, windows)))]
pub(super) struct InterruptListener;

impl InterruptListener {
    #[cfg(unix)]
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            signal: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .context("install remote status interrupt handler")?,
        })
    }

    #[cfg(windows)]
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            signal: tokio::signal::windows::ctrl_c()
                .context("install remote status interrupt handler")?,
        })
    }

    #[cfg(not(any(unix, windows)))]
    pub(super) fn new() -> Result<Self> {
        Ok(Self)
    }

    #[cfg(unix)]
    async fn wait_for_interrupt(&mut self) -> Result<()> {
        self.signal.recv().await;
        Ok(())
    }

    #[cfg(windows)]
    async fn wait_for_interrupt(&mut self) -> Result<()> {
        self.signal.recv().await;
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    async fn wait_for_interrupt(&mut self) -> Result<()> {
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}

impl InterruptSource for InterruptListener {
    fn interrupted(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
        Box::pin(self.wait_for_interrupt())
    }
}

pub(super) async fn wait_for_next_poll<I>(
    interrupt: &mut I,
    poll_interval: Duration,
) -> Result<bool>
where
    I: InterruptSource + ?Sized,
{
    tokio::select! {
        _ = sleep(poll_interval) => Ok(false),
        signal = interrupt.interrupted() => {
            signal.context("wait for remote status interrupt")?;
            Ok(true)
        }
    }
}
