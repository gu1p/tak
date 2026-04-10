use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

use super::RenderMode;

const ANSI_RESET: &str = "\x1b[0m";

pub(crate) fn render_lines(lines: &[Line<'static>], mode: RenderMode) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str(&render_line(line, mode));
        output.push('\n');
    }
    output
}

fn render_line(line: &Line<'static>, mode: RenderMode) -> String {
    let mut output = String::new();
    for span in line.spans.iter() {
        match mode {
            RenderMode::Plain => output.push_str(span.content.as_ref()),
            RenderMode::Ansi => {
                let prefix = style_prefix(span.style);
                if prefix.is_empty() {
                    output.push_str(span.content.as_ref());
                } else {
                    output.push_str(&prefix);
                    output.push_str(span.content.as_ref());
                    output.push_str(ANSI_RESET);
                }
            }
        }
    }
    output
}

fn style_prefix(style: Style) -> String {
    let mut codes = Vec::new();
    if style.add_modifier.contains(Modifier::BOLD) {
        codes.push("1");
    }
    if style.add_modifier.contains(Modifier::DIM) {
        codes.push("2");
    }
    match style.fg {
        Some(Color::Red) => codes.push("31"),
        Some(Color::Yellow) => codes.push("33"),
        Some(Color::Gray) => codes.push("37"),
        Some(Color::Cyan) => codes.push("36"),
        _ => {}
    }
    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}
