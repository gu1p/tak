use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Request, Response, RunLifecycleState};

#[path = "attach/dashboard.rs"]
mod dashboard;
#[path = "attach/interrupt.rs"]
mod interrupt;

pub(super) use interrupt::handle_interrupt;

pub(super) async fn run_with_interrupts(
    socket_path: &Path,
    run_id: &str,
    mut interrupts: crate::cli::attachment_interrupt::State,
    checkout: &crate::cli::run_checkout_store::CheckoutContext,
    renderer: Option<&dyn super::super::PersistedEventRenderer>,
    dashboard: &mut Option<crate::cli::run_dashboard::RunDashboard>,
) -> Result<ExitCode> {
    let mut after_event = 0;
    let mut reported_expired_logs = false;
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
                if handle_interrupt(
                    socket_path, run_id, action?, &mut interrupts, dashboard, renderer,
                ).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
                continue;
            }
            input = next_dashboard_interrupt(dashboard.as_mut()) => {
                if !dashboard::input(
                    dashboard, renderer, input, "during run attachment input",
                ) {
                    continue;
                }
                if handle_interrupt(
                    socket_path, run_id, interrupts.record(), &mut interrupts,
                    dashboard, renderer,
                ).await? {
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
            bail!("local takd returned an unexpected AttachRun response")
        };
        crate::cli::runs_cli::attach::validate_event_page(
            crate::cli::runs_cli::attach::EventPage {
                expected_run: run_id,
                response_run: &response_run,
                after_event,
                events: &events,
                next_event,
                state,
                terminal,
                logs_expired,
            },
        )?;
        if logs_expired && !reported_expired_logs {
            let displayed = dashboard::attempt(
                dashboard,
                renderer,
                |dashboard| dashboard.note_logs_expired(),
                "while reporting expired logs",
            );
            if displayed.is_none() {
                eprintln!("Run logs have expired.");
            }
            reported_expired_logs = true;
        }
        for event in &events {
            let dashboard_handled = dashboard::attempt(
                dashboard,
                renderer,
                |dashboard| dashboard.render_event(event),
                "while rendering a run event",
            )
            .unwrap_or(false);
            let renderer_handled = renderer.map_or(Ok(false), |renderer| renderer.render(event))?;
            if !dashboard_handled && !renderer_handled {
                super::render::event(event)?;
                dashboard::attempt(
                    dashboard,
                    renderer,
                    |dashboard| dashboard.refresh(),
                    "while refreshing redirected output",
                );
            }
        }
        dashboard::attempt(
            dashboard,
            renderer,
            |dashboard| dashboard.render_page_state(state),
            "while updating run state",
        );
        after_event = next_event;
        if terminal {
            if !matches!(
                state,
                RunLifecycleState::Succeeded
                    | RunLifecycleState::Failed
                    | RunLifecycleState::Cancelled
            ) {
                bail!("local takd returned an invalid terminal run state")
            }
            dashboard::attempt(
                dashboard,
                renderer,
                |dashboard| dashboard.finish(state, None),
                "while finishing the run dashboard",
            );
            if let Err(error) =
                crate::cli::output_materialization::materialize(socket_path, run_id, checkout).await
            {
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
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            action = interrupts.next() => {
                if handle_interrupt(
                    socket_path, run_id, action?, &mut interrupts, dashboard, renderer,
                ).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
            }
            input = next_dashboard_interrupt(dashboard.as_mut()) => {
                if !dashboard::input(
                    dashboard, renderer, input, "during run attachment input",
                ) {
                    continue;
                }
                if handle_interrupt(
                    socket_path, run_id, interrupts.record(), &mut interrupts,
                    dashboard, renderer,
                ).await? {
                    bail!("detached from run {run_id}; persisted cancellation continues")
                }
            }
        }
    }
}

pub(super) async fn next_dashboard_interrupt(
    dashboard: Option<&mut crate::cli::run_dashboard::RunDashboard>,
) -> Result<()> {
    match dashboard {
        Some(dashboard) => dashboard.next_interrupt().await,
        None => std::future::pending().await,
    }
}
