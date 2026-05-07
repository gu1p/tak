use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Widget, Wrap};

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

pub(super) fn wrapped_text_height(text: &str, width: u16) -> u16 {
    let area = Rect::new(0, 0, width.max(1), measure_height(text));
    let mut buffer = Buffer::empty(area);
    Paragraph::new(Text::from(text.to_string()))
        .wrap(Wrap { trim: false })
        .render(area, &mut buffer);
    rendered_text_height(&buffer).max(1)
}

fn rendered_text_height(buffer: &Buffer) -> u16 {
    let area = buffer.area;
    let mut last_non_empty_row = 0;
    for y in area.y..(area.y + area.height) {
        let row_has_content = (area.x..(area.x + area.width)).any(|x| {
            let symbol = buffer[(x, y)].symbol();
            !symbol.is_empty() && symbol != " "
        });
        if row_has_content {
            last_non_empty_row = y - area.y + 1;
        }
    }
    last_non_empty_row
}

fn measure_height(text: &str) -> u16 {
    let line_count = text.lines().count().max(1);
    let char_count = text.chars().count().max(1);
    line_count.saturating_add(char_count).min(u16::MAX as usize) as u16
}
