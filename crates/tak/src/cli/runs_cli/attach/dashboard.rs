use std::path::Path;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Response};

use super::super::{MISMATCH_DIAGNOSTIC, request};
use crate::cli::run_dashboard::{DashboardSeed, RunDashboard};

pub(super) async fn load(socket: &Path, run_id: &str) -> Result<Option<RunDashboard>> {
    if !RunDashboard::wanted() {
        return Ok(None);
    }
    let response = request(
        socket,
        "tak-runs-attach-dashboard",
        Operation::GetRun {
            run_id: run_id.to_owned(),
        },
        false,
    )
    .await?;
    let Response::RunDetails { run, .. } = response else {
        bail!(MISMATCH_DIAGNOSTIC)
    };
    if run.summary.run_id != run_id {
        bail!(MISMATCH_DIAGNOSTIC);
    }
    Ok(crate::cli::run_dashboard::start_or_disable(
        RunDashboard::detect(DashboardSeed::from(&run)),
        "before attaching to the run",
    ))
}

pub(super) async fn next_interrupt(dashboard: Option<&mut RunDashboard>) -> Result<()> {
    match dashboard {
        Some(dashboard) => dashboard.next_interrupt().await,
        None => std::future::pending().await,
    }
}

pub(super) fn attempt<U>(
    dashboard: &mut Option<RunDashboard>,
    operation: impl FnOnce(&mut RunDashboard) -> Result<U>,
    stage: &str,
) -> Option<U> {
    crate::cli::run_dashboard::attempt_or_disable(dashboard, operation, stage)
}

pub(super) fn input(dashboard: &mut Option<RunDashboard>, input: Result<()>, stage: &str) -> bool {
    crate::cli::run_dashboard::input_or_disable(dashboard, input, stage)
}
