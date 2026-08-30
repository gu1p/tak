use std::path::Path;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Response, RunEvent, RunLifecycleState};

use super::{MISMATCH_DIAGNOSTIC, render, request};

pub(super) async fn run(socket: &Path, run_id: &str) -> Result<()> {
    let mut after_event = 0;
    let mut interrupts = crate::cli::attachment_interrupt::State::new()?;
    loop {
        let attached = request(
            socket,
            "tak-runs-attach",
            Operation::AttachRun {
                run_id: run_id.to_owned(),
                after_event,
            },
            false,
        );
        let response = tokio::select! {
            response = attached => response?,
            action = interrupts.next() => {
                if handle_interrupt(socket, run_id, action?, &mut interrupts).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
                continue;
            }
        };
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
        render::events(&events)?;
        after_event = next_event;
        if terminal {
            return match state {
                RunLifecycleState::Succeeded => materialize(socket, run_id).await,
                RunLifecycleState::Failed | RunLifecycleState::Cancelled => {
                    bail!("run {run_id} did not succeed")
                }
                _ => bail!(MISMATCH_DIAGNOSTIC),
            };
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            action = interrupts.next() => {
                if handle_interrupt(socket, run_id, action?, &mut interrupts).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
            }
        }
    }
}

async fn materialize(socket: &Path, run_id: &str) -> Result<()> {
    let store = crate::cli::run_checkout_store::RunCheckoutStore::open_default()?;
    if let Some(context) = store.load(socket, run_id)? {
        return crate::cli::output_materialization::materialize(socket, run_id, &context).await;
    }
    let bundle = crate::cli::runs_cli::outputs::fetch(socket, run_id).await?;
    if bundle.manifest.entries.is_empty() {
        return Ok(());
    }
    bail!(
        "original checkout association for run {run_id} is unavailable; outputs remain in takd. Use `tak runs outputs {run_id} --to DIR`."
    )
}

async fn handle_interrupt(
    socket: &Path,
    run_id: &str,
    action: crate::cli::attachment_interrupt::Action,
    interrupts: &mut crate::cli::attachment_interrupt::State,
) -> Result<bool> {
    use crate::cli::attachment_interrupt::Action;
    if matches!(action, Action::Detach) {
        return Ok(true);
    }
    let cancellation = request(
        socket,
        "tak-runs-attach-cancel",
        Operation::CancelRun {
            run_id: run_id.to_owned(),
        },
        true,
    );
    tokio::pin!(cancellation);
    let mut detach_requested = false;
    let response = loop {
        tokio::select! {
            response = &mut cancellation => break response?,
            action = interrupts.next(), if !detach_requested => {
                detach_requested = matches!(action?, Action::Detach);
            }
        }
    };
    use crate::cli::attachment_interrupt::CancellationOutcome;
    match crate::cli::attachment_interrupt::validate_cancellation(run_id, &response)? {
        CancellationOutcome::Persisted => {
            eprintln!("Cancellation persisted for {run_id}; waiting for takd to stop active work.");
            Ok(detach_requested)
        }
        CancellationOutcome::AlreadyTerminal => {
            eprintln!("Run {run_id} was already terminal; loading its final state.");
            Ok(false)
        }
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
