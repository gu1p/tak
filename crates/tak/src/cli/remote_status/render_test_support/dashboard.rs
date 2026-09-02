use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::super::render_dashboard;
use crate::cli::remote_status::view::RemoteStatusView;

pub(in super::super) fn render_dashboard_text(
    view: &RemoteStatusView,
    color_enabled: bool,
) -> String {
    buffer_to_plain_text(&render_dashboard_buffer(view, color_enabled))
}

fn buffer_to_plain_text(buffer: &Buffer) -> String {
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

pub(in super::super) fn render_dashboard_buffer(
    view: &RemoteStatusView,
    color_enabled: bool,
) -> Buffer {
    let backend = TestBackend::new(118, 34);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_dashboard(frame, view, color_enabled))
        .expect("draw dashboard");
    terminal.backend().buffer().clone()
}

pub(in super::super) fn style_for_text(buffer: &Buffer, needle: &str) -> ratatui::style::Style {
    let area = buffer.area;
    for y in area.y..(area.y + area.height) {
        let mut row = String::with_capacity(area.width as usize);
        for x in area.x..(area.x + area.width) {
            row.push_str(buffer[(x, y)].symbol());
        }
        if let Some(column) = row.find(needle) {
            let x = area.x + u16::try_from(column).expect("needle column fits in u16");
            return buffer[(x, y)].style();
        }
    }
    panic!("missing {needle:?} in dashboard buffer");
}
