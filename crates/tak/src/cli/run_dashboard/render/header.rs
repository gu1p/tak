use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::super::model::{DashboardState, JobActivity};
use super::{enabled, lifecycle_style, text};

pub(super) fn header(state: &DashboardState, color: bool, width: u16) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            " TAK RUN  ",
            enabled(
                Style::new().add_modifier(ratatui::style::Modifier::BOLD),
                color,
            ),
        ),
        Span::styled(
            state.run_id.clone(),
            enabled(Style::new().fg(Color::DarkGray), color),
        ),
    ])];
    let active = if state.max_parallel_jobs == 0 {
        format!("{} active", state.active_jobs())
    } else {
        format!("{}/{} active", state.active_jobs(), state.max_parallel_jobs)
    };
    let mut fields = vec![(
        state.lifecycle.to_uppercase(),
        lifecycle_style(&state.lifecycle),
    )];
    if !state.jobs.is_empty() {
        fields.extend([
            (
                format!("{}/{} complete", state.terminal_jobs(), state.jobs.len()),
                Style::default(),
            ),
            (active, Style::default()),
            (
                format!("{} queued", state.scheduler_queue().len()),
                Style::default(),
            ),
        ]);
        let failed = state
            .jobs
            .values()
            .filter(|job| job.activity == JobActivity::Failed)
            .count();
        if failed > 0 {
            fields.push((format!("{failed} failed"), lifecycle_style("failed")));
        }
    }
    lines.extend(summary(fields, width, color));
    if let Some(notice) = &state.notice {
        lines.extend(text::lines(
            notice,
            width,
            enabled(Style::new().fg(Color::Yellow), color),
        ));
    }
    lines
}

fn summary(fields: Vec<(String, Style)>, width: u16, color: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut row = Line::from(" ");
    for (label, style) in fields {
        if row.width() > 1
            && row.width() + text::width(&label) + 3 > usize::from(width.saturating_sub(1))
        {
            lines.push(row);
            row = Line::from(" ");
        }
        if row.width() > 1 {
            row.push_span(" · ");
        }
        row.push_span(Span::styled(label, enabled(style, color)));
    }
    lines.push(row);
    lines
}
