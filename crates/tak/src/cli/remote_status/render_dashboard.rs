use ratatui::layout::Margin;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::super::view::RemoteStatusView;

#[path = "render_dashboard_rows.rs"]
mod rows;

#[cfg(test)]
use ratatui::buffer::Buffer;

use rows::{node_line, push_job_lines};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub(in crate::cli::remote_status) fn render_dashboard(
    frame: &mut ratatui::Frame<'_>,
    view: &RemoteStatusView,
    color_enabled: bool,
) {
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(enabled_style(STYLE_TITLE, color_enabled))
        .title(Span::styled(
            " Remote Status ",
            enabled_style(STYLE_TITLE, color_enabled),
        ));
    let inner = block.inner(area).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(dashboard_lines(view, color_enabled)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn dashboard_lines(view: &RemoteStatusView, color_enabled: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        dashboard_header(view, color_enabled),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Nodes",
            enabled_style(STYLE_TITLE, color_enabled),
        )]),
    ];

    for (index, row) in view.rows().iter().enumerate() {
        lines.push(node_line(index, row, view.tick, color_enabled));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Active Jobs",
        enabled_style(STYLE_TITLE, color_enabled),
    )]));
    push_job_lines(&mut lines, view);
    lines
}

fn dashboard_header(view: &RemoteStatusView, color_enabled: bool) -> Line<'static> {
    let mode = if view.watch { "watch" } else { "once" };
    Line::from(vec![
        Span::styled("Remote Status", enabled_style(STYLE_TITLE, color_enabled)),
        Span::raw(format!(
            "  mode={mode} poll={} nodes={} completed={} checking={}",
            view.poll_index,
            view.total_count(),
            view.completed_count(),
            view.checking_count()
        )),
    ])
}

pub(super) fn title_style(color_enabled: bool) -> Style {
    enabled_style(STYLE_TITLE, color_enabled)
}

pub(super) fn enabled_style(style: Style, enabled: bool) -> Style {
    if enabled { style } else { Style::new() }
}

#[cfg(test)]
pub(super) fn buffer_to_plain_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::with_capacity(area.height as usize);
    for y in area.y..(area.y + area.height) {
        let mut line = String::with_capacity(area.width as usize);
        for x in area.x..(area.x + area.width) {
            let symbol = buffer[(x, y)].symbol();
            if symbol.is_empty() {
                line.push(' ');
            } else {
                line.push_str(symbol);
            }
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}
