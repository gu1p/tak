use std::future::{Future, pending};
use std::pin::Pin;

use anyhow::Result;

use super::interrupt::{InterruptSource, PollOutcome, interruptible};

#[tokio::test]
async fn interruptible_poll_returns_interrupted_while_operation_is_pending() -> Result<()> {
    let mut interrupt = ReadyInterrupt;

    let outcome = interruptible(&mut interrupt, pending::<Result<()>>()).await?;

    assert!(matches!(outcome, PollOutcome::Interrupted));
    Ok(())
}

struct ReadyInterrupt;

impl InterruptSource for ReadyInterrupt {
    fn interrupted(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
        Box::pin(async { Ok(()) })
    }
}
