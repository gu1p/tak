use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;

use super::model::DashboardState;
use super::navigation::DashboardNavigation;

#[path = "render/header.rs"]
mod header;
#[path = "render/layout.rs"]
mod layout;
#[path = "render/panels.rs"]
mod panels;
#[path = "render/sections.rs"]
mod sections;
#[path = "render/text.rs"]
mod text;

const TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub(super) fn draw_with_navigation(
    frame: &mut ratatui::Frame<'_>,
    state: &DashboardState,
    navigation: &DashboardNavigation,
    color_enabled: bool,
) {
    let area = frame.area();
    let header = header::header(state, color_enabled, area.width);
    let footer = sections::footer(area.width);
    let areas = Layout::vertical([
        Constraint::Length(height(header.len())),
        Constraint::Min(0),
        Constraint::Length(height(footer.len())),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(header), areas[0]);
    frame.render_widget(
        Paragraph::new(footer).style(enabled(Style::new().fg(Color::DarkGray), color_enabled)),
        areas[2],
    );
    if state.jobs.is_empty() {
        frame.render_widget(
            Paragraph::new(layout::empty_state(state, color_enabled, area.width)),
            areas[1],
        );
        return;
    }
    panels::draw_panels(frame, areas[1], state, navigation, color_enabled);
}

fn height(lines: usize) -> u16 {
    u16::try_from(lines).unwrap_or(u16::MAX)
}

pub(super) fn enabled(style: Style, color_enabled: bool) -> Style {
    if color_enabled {
        style
    } else {
        Style::default()
    }
}

pub(super) fn lifecycle_style(value: &str) -> Style {
    match value {
        "succeeded" => Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        "failed" => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        "cancelling" | "cancelled" => Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        _ => TITLE,
    }
}
