use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub(super) fn render_lines(
    frame: &mut ratatui::Frame<'_>,
    title: &'static str,
    lines: Vec<Line<'static>>,
    style: Style,
) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(area.height), Constraint::Min(0)])
        .split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(Span::styled(title, style));
    let inner = block.inner(rows[0]).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(block, rows[0]);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
