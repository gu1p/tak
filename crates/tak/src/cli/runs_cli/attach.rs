use std::path::Path;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Response, RunEvent, RunLifecycleState};

use super::{MISMATCH_DIAGNOSTIC, render, request};

pub(super) async fn run(socket: &Path, run_id: &str) -> Result<()> {
    let mut after_event = 0;
    loop {
        let response = request(
            socket,
            "tak-runs-attach",
            Operation::AttachRun {
                run_id: run_id.to_owned(),
                after_event,
            },
            false,
        )
        .await?;
        let Response::RunEvents {
            run_id: response_run,
            events,
            next_event,
            state,
            terminal,
            ..
        } = response
        else {
            bail!(MISMATCH_DIAGNOSTIC)
        };
        validate_event_page(
            run_id,
            &response_run,
            after_event,
            &events,
            next_event,
            state,
            terminal,
        )?;
        render::events(&events);
        after_event = next_event;
        if terminal {
            return match state {
                RunLifecycleState::Succeeded => Ok(()),
                RunLifecycleState::Failed | RunLifecycleState::Cancelled => {
                    bail!("run {run_id} did not succeed")
                }
                _ => bail!(MISMATCH_DIAGNOSTIC),
            };
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub(crate) fn validate_event_page(
    expected_run: &str,
    response_run: &str,
    after_event: u64,
    events: &[RunEvent],
    next_event: u64,
    state: RunLifecycleState,
    terminal: bool,
) -> Result<()> {
    let sequences_valid = events.iter().all(|event| event.seq > after_event)
        && events.first().is_none_or(|event| {
            after_event
                .checked_add(1)
                .is_some_and(|expected| event.seq == expected)
        })
        && events.windows(2).all(|pair| pair[0].seq < pair[1].seq)
        && events
            .last()
            .map_or(next_event == after_event, |event| event.seq == next_event);
    if response_run != expected_run || !sequences_valid || (terminal && !state.is_terminal()) {
        bail!(MISMATCH_DIAGNOSTIC);
    }
    Ok(())
}
