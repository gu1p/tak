use ratatui::style::{Color, Style};
use ratatui::text::Line;

use super::super::model::{DashboardJob, DashboardState, JobActivity};
use super::enabled;

#[path = "sections/nodes.rs"]
mod node_lanes;
#[path = "sections/output.rs"]
mod output;
#[path = "sections/tasks_compact.rs"]
mod tasks_compact;
#[path = "sections/tasks_table.rs"]
mod tasks_table;

pub(super) fn nodes(
    state: &DashboardState,
    color: bool,
    width: u16,
    expanded: bool,
) -> Vec<Line<'static>> {
    node_lanes::nodes(state, color, width, expanded)
}

pub(super) fn tasks(state: &DashboardState, color: bool, width: u16) -> Vec<Line<'static>> {
    let mut jobs = state.jobs.iter().collect::<Vec<_>>();
    jobs.sort_by(|(left_id, left), (right_id, right)| {
        activity_priority(left.activity)
            .cmp(&activity_priority(right.activity))
            .then_with(|| left_id.cmp(right_id))
    });
    if width < 78 {
        tasks_compact::tasks(jobs, color, width)
    } else {
        tasks_table::tasks(jobs, color, width)
    }
}

pub(super) fn scheduler_queue(
    state: &DashboardState,
    color: bool,
    width: u16,
) -> Vec<Line<'static>> {
    output::scheduler_queue(state, color, width)
}

pub(super) fn logs(state: &DashboardState, color: bool, width: u16) -> Vec<Line<'static>> {
    output::logs(state, color, width)
}

pub(super) fn footer(width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut row = String::new();
    for hint in [
        "Tab panel",
        "↑↓ scroll",
        "PgUp/PgDn",
        "Home/End",
        "Ctrl-C cancel/again detach",
    ] {
        if !row.is_empty()
            && super::text::width(&row) + super::text::width(hint) + 3
                > usize::from(width.saturating_sub(2))
        {
            lines.push(Line::from(format!(" {row}")));
            row.clear();
        }
        if !row.is_empty() {
            row.push_str(" · ");
        }
        row.push_str(hint);
    }
    lines.extend(super::text::lines(&row, width, Style::default()));
    lines
}

fn task_name(job: &DashboardJob) -> String {
    let Some(first) = job.task_ids.first() else {
        return "(unnamed job)".into();
    };
    let extra = job.task_ids.len().saturating_sub(1);
    if extra == 0 {
        first.clone()
    } else {
        format!("{first} (+{extra})")
    }
}

fn activity_style(activity: JobActivity) -> Style {
    match activity {
        JobActivity::Staging => Style::new().fg(Color::DarkGray),
        JobActivity::Ready | JobActivity::Retrying => Style::new().fg(Color::Yellow),
        JobActivity::Succeeded => Style::new().fg(Color::Green),
        JobActivity::Running => Style::new().fg(Color::Cyan),
        JobActivity::Failed => Style::new().fg(Color::Red),
        JobActivity::Transferring | JobActivity::OutputCommitting => Style::new().fg(Color::Cyan),
        JobActivity::Cancelling | JobActivity::Cancelled | JobActivity::Skipped => {
            Style::new().fg(Color::DarkGray)
        }
        JobActivity::Blocked => Style::new().fg(Color::DarkGray),
        JobActivity::Unknown => Style::new().fg(Color::Yellow),
    }
}

fn activity_symbol(activity: JobActivity) -> &'static str {
    match activity {
        JobActivity::Running => "●",
        JobActivity::Succeeded => "✓",
        JobActivity::Failed => "✕",
        JobActivity::Cancelled | JobActivity::Skipped => "–",
        JobActivity::Staging | JobActivity::Blocked | JobActivity::Unknown => "○",
        _ => "◷",
    }
}

fn activity_priority(activity: JobActivity) -> u8 {
    if activity.is_active() {
        return 0;
    }
    match activity {
        JobActivity::Failed => 1,
        JobActivity::Ready | JobActivity::Retrying => 2,
        JobActivity::Staging | JobActivity::Blocked | JobActivity::Unknown => 3,
        JobActivity::Succeeded
        | JobActivity::Cancelled
        | JobActivity::Skipped
        | JobActivity::Transferring
        | JobActivity::Running
        | JobActivity::OutputCommitting
        | JobActivity::Cancelling => 4,
    }
}
