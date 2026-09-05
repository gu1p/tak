use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::super::super::model::{DashboardState, NodeLane};
use super::super::{enabled, text};

pub(super) fn nodes(
    state: &DashboardState,
    color: bool,
    width: u16,
    expanded: bool,
) -> Vec<Line<'static>> {
    if state.nodes.is_empty() {
        return text::lines("No node candidates for this run", width, Style::default());
    }
    let mut nodes = state.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|(left_id, left), (right_id, right)| {
        right
            .active_jobs
            .len()
            .cmp(&left.active_jobs.len())
            .then_with(|| left_id.cmp(right_id))
    });
    let mut lines = Vec::new();
    for (node, lane) in nodes {
        let label = format!(
            "{node} · {} active task{} · {} candidate{}",
            lane.active_jobs.len(),
            if lane.active_jobs.len() == 1 { "" } else { "s" },
            lane.candidate_queue.len(),
            if lane.candidate_queue.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        lines.extend(text::lines(
            &label,
            width,
            enabled(Style::new().add_modifier(Modifier::BOLD), color),
        ));
        if expanded {
            push_details(&mut lines, lane, color, width);
        }
    }
    lines
}

fn push_details(lines: &mut Vec<Line<'static>>, lane: &NodeLane, color: bool, width: u16) {
    for task in &lane.active_jobs {
        push_row(
            lines,
            "ACTIVE",
            task,
            width,
            enabled(Style::new().fg(Color::Cyan), color),
        );
    }
    for entry in &lane.candidate_queue {
        push_row(
            lines,
            "CANDIDATE QUEUE",
            &format!(
                "{} · queue={}",
                entry.task,
                entry.queue.as_deref().unwrap_or("none")
            ),
            width,
            enabled(Style::new().fg(Color::Yellow), color),
        );
    }
}

fn push_row(lines: &mut Vec<Line<'static>>, label: &str, value: &str, width: u16, style: Style) {
    let prefix = format!("   {label}  ");
    if text::width(&prefix) + text::width(value) <= usize::from(width) {
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::raw(value.to_owned()),
        ]));
    } else {
        lines.push(Line::from(Span::styled(format!("   {label}"), style)));
        lines.extend(text::lines(value, width, Style::default()));
    }
}
