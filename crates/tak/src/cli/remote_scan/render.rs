use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::{ScanApp, Screen};
use super::provider::GrayFrame;

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &ScanApp) {
    match app.screen {
        Screen::Picker => render_picker(frame, app),
        Screen::Preview => render_preview(frame, app),
        Screen::Confirm => render_confirm(frame, app),
    }
}

pub(super) fn to_text(app: &ScanApp) -> Result<String> {
    let backend = TestBackend::new(96, 36);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, app))?;
    Ok(buffer_to_plain_text(terminal.backend().buffer()))
}

fn render_picker(frame: &mut ratatui::Frame<'_>, app: &ScanApp) {
    let body = app
        .cameras
        .iter()
        .enumerate()
        .map(|(index, camera)| format!("{} {}", marker(app, index), camera.label))
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("Use Up/Down then Enter.\n\n{body}");
    render_block(frame, "Choose Camera", &text, None);
}

fn render_preview(frame: &mut ratatui::Frame<'_>, app: &ScanApp) {
    let preview = preview_text(app.preview.as_ref(), 72, 22);
    let text = format!(
        "Camera: {}\nEsc back, q quit.\n\n{}",
        app.cameras[app.selected].label, preview
    );
    render_block(frame, "Scan QR Code", &text, None);
}

fn render_confirm(frame: &mut ratatui::Frame<'_>, app: &ScanApp) {
    let found = app.detected.as_ref().expect("confirm view needs match");
    let preview = preview_text(app.preview.as_ref(), 72, 12);
    let details = [
        format!("Display: {}", found.display_name),
        format!("Node: {}", found.node_id),
        format!("Base URL: {}", found.base_url),
        format!("Transport: {}", found.transport),
        String::new(),
        "Enter add, Esc back, q quit.".to_string(),
        String::new(),
        preview,
    ]
    .join("\n");
    render_block(frame, "Confirm Remote", &details, Some(24));
}

fn render_block(frame: &mut ratatui::Frame<'_>, title: &str, text: &str, top: Option<u16>) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top.unwrap_or(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title));
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn preview_text(frame: Option<&GrayFrame>, width: usize, height: usize) -> String {
    let Some(frame) = frame else {
        return "(waiting for camera frame)".to_string();
    };
    let palette = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    let mut lines = Vec::with_capacity(height);
    for y in 0..height {
        let mut line = String::with_capacity(width);
        for x in 0..width {
            let src_x = x * frame.width as usize / width.max(1);
            let src_y = y * frame.height as usize / height.max(1);
            let value = frame.pixels[src_y * frame.width as usize + src_x] as usize;
            let index = value * (palette.len() - 1) / 255;
            line.push(palette[palette.len() - 1 - index]);
        }
        lines.push(line);
    }
    lines.join("\n")
}

fn marker(app: &ScanApp, index: usize) -> &'static str {
    if app.selected == index { ">" } else { " " }
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
