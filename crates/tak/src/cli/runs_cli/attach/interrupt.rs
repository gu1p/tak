use std::path::Path;

use anyhow::Result;
use tak_proto::local_daemon::v2::Operation;

use super::super::request;

pub(super) async fn handle_interrupt(
    socket: &Path,
    run_id: &str,
    action: crate::cli::attachment_interrupt::Action,
    interrupts: &mut crate::cli::attachment_interrupt::State,
    dashboard: &mut Option<crate::cli::run_dashboard::RunDashboard>,
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
            input = super::dashboard::next_interrupt(dashboard.as_mut()), if !detach_requested => {
                if super::dashboard::input(dashboard, input, "while persisting cancellation") {
                    detach_requested = matches!(interrupts.record(), Action::Detach);
                }
            }
        }
    };
    use crate::cli::attachment_interrupt::CancellationOutcome;
    match crate::cli::attachment_interrupt::validate_cancellation(run_id, &response)? {
        CancellationOutcome::Persisted => {
            let displayed = super::dashboard::attempt(
                dashboard,
                |dashboard| dashboard.note_cancellation_persisted(),
                "while reporting persisted cancellation",
            );
            if displayed.is_none() {
                eprintln!(
                    "Cancellation persisted for {run_id}; waiting for takd to stop active work."
                );
            }
            Ok(detach_requested)
        }
        CancellationOutcome::AlreadyTerminal => {
            let displayed = super::dashboard::attempt(
                dashboard,
                |dashboard| dashboard.note_already_terminal(),
                "while reporting terminal cancellation",
            );
            if displayed.is_none() {
                eprintln!("Run {run_id} was already terminal; loading its final state.");
            }
            Ok(false)
        }
    }
}
