use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Response, RunLifecycleState};

use super::{MISMATCH_DIAGNOSTIC, render, request};

#[path = "attach/validation.rs"]
mod validation;

pub(crate) use validation::{EventPage, validate_event_page};

pub(super) async fn run(socket: &Path, run_id: &str) -> Result<ExitCode> {
    let mut after_event = 0;
    let mut reported_expired_logs = false;
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
            logs_expired,
            exit_code,
            ..
        } = response
        else {
            bail!(MISMATCH_DIAGNOSTIC)
        };
        validate_event_page(EventPage {
            expected_run: run_id,
            response_run: &response_run,
            after_event,
            events: &events,
            next_event,
            state,
            terminal,
            logs_expired,
        })?;
        if logs_expired && !reported_expired_logs {
            eprintln!("Run logs have expired.");
            reported_expired_logs = true;
        }
        render::events(&events)?;
        after_event = next_event;
        if terminal {
            if !matches!(
                state,
                RunLifecycleState::Succeeded
                    | RunLifecycleState::Failed
                    | RunLifecycleState::Cancelled
            ) {
                bail!(MISMATCH_DIAGNOSTIC)
            }
            if let Err(error) = materialize(socket, run_id).await {
                if state == RunLifecycleState::Succeeded {
                    return Err(error);
                }
                eprintln!("run {run_id} output materialization failed: {error:#}");
            }
            return match state {
                RunLifecycleState::Succeeded => Ok(ExitCode::SUCCESS),
                RunLifecycleState::Failed => match exit_code {
                    Some(code) => {
                        eprintln!("run {run_id} failed with exit code {code}");
                        Ok(ExitCode::from(code as u8))
                    }
                    None => bail!("run {run_id} did not succeed"),
                },
                RunLifecycleState::Cancelled => bail!("run {run_id} was cancelled"),
                _ => unreachable!("terminal state was validated"),
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
    let bundle = crate::cli::runs_cli::outputs::fetch_foreground(socket, run_id).await?;
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
