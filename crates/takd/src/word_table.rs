use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use tak_proto::encode_tor_invite_words;

const VIEW_WIDTH: u16 = 84;
const BLOCK_VERTICAL_CHROME: u16 = 4;
const BLOCK_HORIZONTAL_CHROME: u16 = 6;

pub(crate) fn render_words_table_view(token: &str) -> Result<String> {
    let phrase = encode_tor_invite_words(token)?;
    let table = numbered_words_text(&phrase);
    let text_width = VIEW_WIDTH.saturating_sub(BLOCK_HORIZONTAL_CHROME).max(1);
    let view_height = wrapped_text_height(&table, text_width) + BLOCK_VERTICAL_CHROME;
    let backend = TestBackend::new(VIEW_WIDTH, view_height);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| {
        let block = Block::default().borders(Borders::ALL).title(" Words ");
        let words_area = block.inner(frame.area()).inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        frame.render_widget(block, frame.area());
        frame.render_widget(
            Paragraph::new(Text::from(table.clone())).wrap(Wrap { trim: false }),
            words_area,
        );
    })?;

    Ok(format!(
        "{}\n",
        buffer_to_plain_text(terminal.backend().buffer())
    ))
}

pub(crate) fn numbered_words_text(phrase: &str) -> String {
    let words = phrase.split_whitespace().collect::<Vec<_>>();
    let max_len = words
        .iter()
        .map(|word| word.len())
        .max()
        .unwrap_or(10)
        .max(10);
    let cell_width = max_len + 5;
    let mut rows = Vec::new();
    for row_start in (0..words.len()).step_by(3) {
        let mut row = String::new();
        for (offset, word) in words[row_start..words.len().min(row_start + 3)]
            .iter()
            .enumerate()
        {
            if offset > 0 {
                row.push_str("  ");
            }
            let index = row_start + offset + 1;
            row.push_str(&format!(
                "{index:02} {word:<width$}",
                width = cell_width - 3
            ));
        }
        rows.push(row.trim_end().to_string());
    }
    rows.join("\n")
}

fn wrapped_text_height(text: &str, width: u16) -> u16 {
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
    u16::try_from(line_count + 32).unwrap_or(u16::MAX)
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
