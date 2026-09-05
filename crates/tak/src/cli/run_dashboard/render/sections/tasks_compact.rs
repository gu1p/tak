use ratatui::style::Style;
use ratatui::text::Line;

use super::super::super::model::DashboardJob;
use super::{activity_style, activity_symbol, enabled, task_name};

pub(super) fn tasks(
    jobs: Vec<(&String, &DashboardJob)>,
    color: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(" STATE · TASK")];
    for (_, job) in jobs {
        let label = format!(
            "{} {} · {}",
            activity_symbol(job.activity),
            job.activity.as_str(),
            task_name(job)
        );
        lines.extend(super::super::text::lines(
            &label,
            width,
            enabled(activity_style(job.activity), color),
        ));
        let metadata = format!(
            "NODE {} · TRY {} · CACHE {}",
            job.node_id.as_deref().unwrap_or("unassigned"),
            attempt(job),
            job.cache.as_deref().unwrap_or("—")
        );
        lines.extend(super::super::text::lines(
            &metadata,
            width,
            Style::default(),
        ));
    }
    lines
}

pub(super) fn attempt(job: &DashboardJob) -> String {
    if job.attempt == 0 {
        "—".into()
    } else {
        job.attempt.to_string()
    }
}
