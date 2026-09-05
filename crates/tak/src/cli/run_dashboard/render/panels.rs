use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::model::DashboardState;
use super::super::navigation::{DashboardNavigation, Panel};
use super::{TITLE, enabled, layout, sections};

pub(super) fn draw_panels(
    frame: &mut ratatui::Frame<'_>,
    mut area: ratatui::layout::Rect,
    state: &DashboardState,
    navigation: &DashboardNavigation,
    color: bool,
) {
    if area.height < 14 {
        let tabs = [
            (Panel::Nodes, "NODES"),
            (Panel::Tasks, "TASKS"),
            (Panel::Queue, "QUEUE"),
            (Panel::Logs, "LOGS"),
        ]
        .into_iter()
        .map(|(panel, name)| {
            Span::styled(
                format!(
                    " {}{name} ",
                    if navigation.focus() == panel {
                        "▶ "
                    } else {
                        ""
                    }
                ),
                enabled(
                    if navigation.focus() == panel {
                        TITLE
                    } else {
                        Style::default()
                    },
                    color,
                ),
            )
        })
        .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(Line::from(tabs)),
            ratatui::layout::Rect {
                height: area.height.min(1),
                ..area
            },
        );
        area.y = area.y.saturating_add(1);
        area.height = area.height.saturating_sub(1);
    }
    let panels = [
        (
            "NODES",
            Panel::Nodes,
            sections::nodes(state, color, area.width, navigation.focus() == Panel::Nodes),
        ),
        (
            "TASKS",
            Panel::Tasks,
            sections::tasks(state, color, area.width),
        ),
        (
            "SCHEDULER QUEUE",
            Panel::Queue,
            sections::scheduler_queue(state, color, area.width),
        ),
        (
            "LIVE LOGS",
            Panel::Logs,
            sections::logs(state, color, area.width),
        ),
    ];
    let heights = layout::panel_heights(
        panels.each_ref().map(|(_, _, lines)| lines.len()),
        area.height,
        navigation.focus(),
    );
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(heights.map(Constraint::Length))
        .split(area);
    for (index, (title, panel, lines)) in panels.into_iter().enumerate() {
        render_panel(frame, areas[index], title, lines, panel, navigation, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_panel(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    title: &str,
    mut lines: Vec<Line<'static>>,
    panel: Panel,
    navigation: &DashboardNavigation,
    color: bool,
) {
    if area.height == 0 {
        return;
    }
    let focused = navigation.focus() == panel;
    let marker = if focused { "▶ " } else { "" };
    let style = enabled(
        if focused {
            TITLE
        } else {
            Style::new().fg(Color::DarkGray)
        },
        color,
    );
    let pinned_header = (panel == Panel::Tasks && area.height > 2).then(|| lines.remove(0));
    let visible = usize::from(
        area.height
            .saturating_sub(1 + u16::from(pinned_header.is_some())),
    );
    let scroll = navigation.scroll_offset(panel, lines.len(), visible);
    let mut block = Block::default()
        .borders(Borders::TOP)
        .border_style(style)
        .title(Span::styled(format!(" {marker}{title} "), style));
    if lines.len() > visible && visible > 0 {
        block = block.title(
            Line::from(format!(
                " {}–{} of {} ",
                usize::from(scroll) + 1,
                (usize::from(scroll) + visible).min(lines.len()),
                lines.len()
            ))
            .right_aligned(),
        );
    }
    let mut content = block.inner(area);
    frame.render_widget(block, area);
    if let Some(header) = pinned_header {
        frame.render_widget(
            Paragraph::new(header).style(enabled(Style::new().fg(Color::DarkGray), color)),
            ratatui::layout::Rect {
                height: 1,
                ..content
            },
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), content);
}
