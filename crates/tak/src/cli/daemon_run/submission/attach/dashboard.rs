use anyhow::Result;

use crate::cli::daemon_run::PersistedEventRenderer;
use crate::cli::run_dashboard::RunDashboard;

pub(super) fn attempt<U>(
    dashboard: &mut Option<RunDashboard>,
    renderer: Option<&dyn PersistedEventRenderer>,
    operation: impl FnOnce(&mut RunDashboard) -> Result<U>,
    stage: &str,
) -> Option<U> {
    let result = crate::cli::run_dashboard::attempt_or_disable(dashboard, operation, stage);
    sync_renderer(renderer, dashboard.is_some());
    result
}

pub(super) fn input(
    dashboard: &mut Option<RunDashboard>,
    renderer: Option<&dyn PersistedEventRenderer>,
    input: Result<()>,
    stage: &str,
) -> bool {
    let is_interrupt = crate::cli::run_dashboard::input_or_disable(dashboard, input, stage);
    sync_renderer(renderer, dashboard.is_some());
    is_interrupt
}

fn sync_renderer(renderer: Option<&dyn PersistedEventRenderer>, active: bool) {
    if let Some(renderer) = renderer {
        renderer.set_dashboard_active(active);
    }
}
