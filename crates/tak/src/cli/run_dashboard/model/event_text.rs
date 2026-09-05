pub(super) fn safe_terminal_text(text: &str) -> String {
    text.chars()
        .filter(|character| *character == '\n' || !character.is_control())
        .collect()
}

pub(super) fn safe_terminal_field(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}
