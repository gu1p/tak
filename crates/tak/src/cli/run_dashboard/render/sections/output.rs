use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::super::super::model::DashboardState;
use super::super::text::{lines, width, wrap};
use super::{activity_style, enabled, task_name};

pub(super) fn scheduler_queue(
    state: &DashboardState,
    color: bool,
    available: u16,
) -> Vec<Line<'static>> {
    let queued = state.scheduler_queue();
    if queued.is_empty() {
        return lines(
            "Empty · no tasks waiting for placement",
            available,
            Style::default(),
        );
    }
    queued
        .into_iter()
        .filter_map(|id| state.jobs.get(id))
        .flat_map(|job| {
            let candidates = if job.candidate_node_ids.is_empty() {
                "unavailable".into()
            } else {
                job.candidate_node_ids.join(", ")
            };
            lines(
                &format!(
                    "{} · queue: {} · candidates: {candidates}",
                    task_name(job),
                    job.queue.as_deref().unwrap_or("none")
                ),
                available,
                enabled(activity_style(job.activity), color),
            )
        })
        .collect()
}

pub(super) fn logs(state: &DashboardState, color: bool, available: u16) -> Vec<Line<'static>> {
    if state.logs.is_empty() {
        return lines("Waiting for task output…", available, Style::default());
    }
    let mut output = Vec::new();
    let available = usize::from(available.saturating_sub(2)).max(1);
    for log in &state.logs {
        let source = format!("{}@{}", log.job, log.node);
        let prefix_width = width(&source) + 3;
        let inline = prefix_width < available / 2;
        if !inline {
            output.extend(wrap(&source, available).into_iter().map(|row| {
                Line::from(Span::styled(
                    format!(" {row}"),
                    enabled(Style::new().fg(Color::DarkGray), color),
                ))
            }));
        }
        let log_width = available.saturating_sub(if inline { prefix_width } else { 0 });
        for row in log.text.lines().flat_map(|line| wrap(line, log_width)) {
            let mut spans = vec![Span::raw(" ")];
            if inline {
                spans.push(Span::styled(
                    format!("{source} │ "),
                    enabled(Style::new().fg(Color::DarkGray), color),
                ));
            }
            spans.push(Span::raw(row));
            output.push(Line::from(spans));
        }
    }
    output
}
