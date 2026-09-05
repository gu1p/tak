use ratatui::style::Style;
use ratatui::text::{Line, Span};

#[path = "text_tests.rs"]
#[cfg(test)]
mod tests;

pub(super) fn width(value: &str) -> usize {
    Span::raw(value).width()
}

pub(super) fn wrap(value: &str, available: usize) -> Vec<String> {
    let available = available.max(1);
    let mut rows = Vec::new();
    for source in value.split('\n') {
        let mut row = String::new();
        for word in source.split_inclusive(' ') {
            if !row.is_empty() && width(&row) + width(word.trim_end()) > available {
                rows.push(row.trim_end().to_owned());
                row.clear();
            }
            for character in word.chars() {
                if !row.is_empty() && width(&row) + width(&character.to_string()) > available {
                    if character == ' ' {
                        continue;
                    }
                    rows.push(std::mem::take(&mut row));
                }
                row.push(character);
            }
        }
        rows.push(row.trim_end().to_owned());
    }
    rows
}

pub(super) fn lines(value: &str, available: u16, style: Style) -> Vec<Line<'static>> {
    wrap(value, usize::from(available.saturating_sub(2)))
        .into_iter()
        .map(|row| Line::from(Span::styled(format!(" {row}"), style)))
        .collect()
}

pub(super) fn padded(value: &str, available: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(available.saturating_sub(width(value)))
    )
}
