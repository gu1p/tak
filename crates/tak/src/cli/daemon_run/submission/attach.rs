use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Request, Response, RunLifecycleState};

pub(super) async fn run_with_interrupts(
    socket_path: &Path,
    run_id: &str,
    mut interrupts: crate::cli::attachment_interrupt::State,
) -> Result<()> {
    let mut after_event = 0;
    loop {
        let request_frame = Request {
            request_id: super::exchange::request_id("attach"),
            operation: Operation::AttachRun {
                run_id: run_id.to_owned(),
                after_event,
            },
        };
        let request = super::exchange::response(socket_path, &request_frame);
        let response = tokio::select! {
            response = request => response?,
            action = interrupts.next() => {
                if handle_interrupt(socket_path, run_id, action?, &mut interrupts).await? {
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
            bail!("local takd returned an unexpected AttachRun response")
        };
        crate::cli::runs_cli::attach::validate_event_page(
            run_id,
            &response_run,
            after_event,
            &events,
            next_event,
            state,
            terminal,
        )?;
        for event in &events {
            super::render::event(event)?;
        }
        after_event = next_event;
        if terminal {
            return match state {
                RunLifecycleState::Succeeded => Ok(()),
                RunLifecycleState::Failed | RunLifecycleState::Cancelled => {
                    bail!("run {run_id} did not succeed")
                }
                _ => bail!("local takd returned an invalid terminal run state"),
            };
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            action = interrupts.next() => {
                if handle_interrupt(socket_path, run_id, action?, &mut interrupts).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
            }
        }
    }
}

pub(super) async fn handle_interrupt(
    socket_path: &Path,
    run_id: &str,
    action: crate::cli::attachment_interrupt::Action,
    interrupts: &mut crate::cli::attachment_interrupt::State,
) -> Result<bool> {
    use crate::cli::attachment_interrupt::Action;
    if matches!(action, Action::Detach) {
        return Ok(true);
    }
    let cancellation_request = Request {
        request_id: super::exchange::request_id("cancel"),
        operation: Operation::CancelRun {
            run_id: run_id.to_owned(),
        },
    };
    let cancellation = super::exchange::response(socket_path, &cancellation_request);
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
