use ratatui::text::{Line, Span};

use super::super::super::model::DashboardJob;
use super::super::text::{padded, width as text_width};
use super::tasks_compact::attempt;
use super::{activity_style, activity_symbol, enabled, task_name};

pub(super) fn tasks(
    jobs: Vec<(&String, &DashboardJob)>,
    color: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let node_width = (usize::from(width) / 5).clamp(16, 24);
    let task_width = usize::from(width).saturating_sub(node_width + 36);
    if jobs.iter().any(|(_, job)| {
        text_width(&task_name(job)) > task_width
            || text_width(job.node_id.as_deref().unwrap_or("unassigned")) > node_width
            || text_width(job.cache.as_deref().unwrap_or("—")) > 7
    }) {
        return super::tasks_compact::tasks(jobs, color, width);
    }
    let mut lines = vec![Line::from(format!(
        " {:<20}{} {} TRY  CACHE",
        "STATE",
        padded("TASK", task_width),
        padded("NODE", node_width)
    ))];
    for (_, job) in jobs {
        let status = format!(
            "{} {}",
            activity_symbol(job.activity),
            job.activity.as_str()
        );
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {}", padded(&status, 20)),
                enabled(activity_style(job.activity), color),
            ),
            Span::raw(format!(
                "{} {} {:>3}  {}",
                padded(&task_name(job), task_width),
                padded(job.node_id.as_deref().unwrap_or("unassigned"), node_width),
                attempt(job),
                job.cache.as_deref().unwrap_or("—")
            )),
        ]));
    }
    lines
}
