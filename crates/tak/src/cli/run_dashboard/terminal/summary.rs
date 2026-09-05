use super::super::model::DashboardState;

const FINAL_LOG_LINE_LIMIT: usize = 8;

pub(in crate::cli::run_dashboard) fn final_summary(state: &DashboardState) -> String {
    let mut lines = vec![format!(
        "tak run {} {} · {}/{} complete",
        state.run_id,
        state.lifecycle,
        state.terminal_jobs(),
        state.jobs.len()
    )];
    lines.extend(
        state
            .diagnostics
            .iter()
            .map(|diagnostic| format!("  failure: {diagnostic}")),
    );
    if let Some(error) = state.error.as_deref().filter(|error| {
        !state
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.ends_with(*error))
    }) {
        lines.push(format!("  failure: run: {error}"));
    }
    if let Some(notice) = &state.notice {
        lines.push(format!("  status: {notice}"));
    }
    let mut recent_logs = state
        .logs
        .iter()
        .rev()
        .flat_map(|log| {
            log.text
                .lines()
                .rev()
                .map(move |text| format!("    {}@{} │ {text}", log.job, log.node))
        })
        .take(FINAL_LOG_LINE_LIMIT)
        .collect::<Vec<_>>();
    if !recent_logs.is_empty() {
        recent_logs.reverse();
        lines.push("  recent output:".into());
        lines.extend(recent_logs);
    }
    lines.join("\n")
}
