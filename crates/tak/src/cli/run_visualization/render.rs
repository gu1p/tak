use std::collections::BTreeMap;

use ratatui::style::Color;

use super::model::{RunState, TaskActivity, TaskRow};
use super::render_selection::visible_rows;
pub(super) use super::render_style::{activity_color, state_name};
use super::render_style::{paint, symbol};
use super::render_text::{
    activity_text, elapsed, fit, footer, placement, placement_node, task_name,
};

pub(super) fn render_plain(state: &RunState, width: usize) -> String {
    render_frame(state, width, false)
}

pub(super) fn render_frame(state: &RunState, width: usize, color: bool) -> String {
    if state.total() == 0 {
        if let Some(error) = state.error.as_deref() {
            return format!(
                "{}\nRun failed: {error}\n",
                paint("tak run · failed", Color::Red, color),
            );
        }
        if state.finished {
            return format!(
                "{}\nNo executable task steps.\n",
                paint("tak run · complete", Color::Green, color),
            );
        }
        return format!(
            "{}\nPlanning execution graph…\n",
            paint("tak run · planning", Color::Cyan, color)
        );
    }
    let all_rows = state.ordered_rows().collect::<Vec<_>>();
    let completed = all_rows
        .iter()
        .filter(|row| row.activity.is_terminal())
        .count();
    let occupied = all_rows
        .iter()
        .filter(|row| row.activity != TaskActivity::Waiting && !row.activity.is_terminal())
        .count();
    let (rows, hidden_completed) = visible_rows(&all_rows);
    let title = format!(
        "tak run  {completed}/{} complete · {}/{} jobs",
        all_rows.len(),
        occupied.min(state.jobs),
        state.jobs,
    );
    let mut output = format!(
        "{}\n{}\n\n",
        paint(&title, Color::Cyan, color),
        fit(&node_strip(&all_rows), width),
    );
    if width < 88 {
        render_stacked(&mut output, &rows, color);
    } else {
        output.push_str(
            "TASK                     PLACEMENT       ACTIVITY                     ELAPSED\n",
        );
        for row in &rows {
            let task = fit(&task_name(row), 24);
            let placement = fit(&placement(row), 15);
            let activity = fit(&activity_text(row), 28);
            let rendered = format!(
                "{} {:<24} {:<15} {:<28} {}\n",
                symbol(row.activity),
                task,
                placement,
                activity,
                elapsed(row)
            );
            output.push_str(&paint(&rendered, activity_color(row.activity), color));
        }
    }
    if hidden_completed > 0 {
        output.push_str(&format!("… {hidden_completed} completed tasks hidden\n"));
    }
    output.push('\n');
    output.push_str(&footer(&all_rows));
    if let Some(error) = state.error.as_deref() {
        output.push_str(&format!("\nRun failed: {error}"));
    }
    output.push('\n');
    output
}

fn render_stacked(output: &mut String, rows: &[&TaskRow], color: bool) {
    for row in rows {
        let rendered = format!(
            "{} {}\n  {} · {}\n",
            symbol(row.activity),
            task_name(row),
            placement(row),
            activity_text(row),
        );
        output.push_str(&paint(&rendered, activity_color(row.activity), color));
    }
}

fn node_strip(rows: &[&TaskRow]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for row in rows.iter().filter(|row| !row.activity.is_terminal()) {
        *counts.entry(placement_node(row)).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(node, count)| format!("{node} ×{count}"))
        .collect::<Vec<_>>()
        .join("  ")
}
