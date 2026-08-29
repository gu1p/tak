pub(super) const MAX_IDENTIFIER_BYTES: usize = 128;

pub(super) fn is_valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_IDENTIFIER_BYTES && !value.chars().any(char::is_control)
}
