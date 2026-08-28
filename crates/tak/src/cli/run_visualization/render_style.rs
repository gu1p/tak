use ratatui::style::Color;

use super::model::TaskActivity;

pub(super) fn activity_color(activity: TaskActivity) -> Color {
    match activity {
        TaskActivity::Waiting | TaskActivity::Cancelled => Color::DarkGray,
        TaskActivity::Placing | TaskActivity::Running => Color::Cyan,
        TaskActivity::Staging | TaskActivity::Syncing => Color::Blue,
        TaskActivity::Uploading => Color::Magenta,
        TaskActivity::Queued | TaskActivity::Retrying => Color::Yellow,
        TaskActivity::Passed => Color::Green,
        TaskActivity::Failed => Color::Red,
    }
}

pub(super) fn paint(value: &str, color: Color, enabled: bool) -> String {
    if !enabled {
        return value.to_string();
    }
    let code = match color {
        Color::DarkGray => 90,
        Color::Red => 31,
        Color::Green => 32,
        Color::Yellow => 33,
        Color::Blue => 34,
        Color::Magenta => 35,
        Color::Cyan => 36,
        _ => 37,
    };
    format!("\u{1b}[{code}m{value}\u{1b}[0m")
}

pub(super) fn symbol(activity: TaskActivity) -> &'static str {
    match activity {
        TaskActivity::Waiting => "○",
        TaskActivity::Placing
        | TaskActivity::Staging
        | TaskActivity::Uploading
        | TaskActivity::Retrying
        | TaskActivity::Syncing
        | TaskActivity::Queued => "◷",
        TaskActivity::Running => "●",
        TaskActivity::Passed => "✓",
        TaskActivity::Failed => "✗",
        TaskActivity::Cancelled => "–",
    }
}

pub(super) fn state_name(activity: TaskActivity) -> &'static str {
    match activity {
        TaskActivity::Waiting => "waiting",
        TaskActivity::Placing => "placing",
        TaskActivity::Staging => "staging",
        TaskActivity::Uploading => "uploading",
        TaskActivity::Queued => "queued",
        TaskActivity::Running => "running",
        TaskActivity::Retrying => "retrying",
        TaskActivity::Syncing => "syncing",
        TaskActivity::Passed => "passed",
        TaskActivity::Failed => "failed",
        TaskActivity::Cancelled => "cancelled",
    }
}
