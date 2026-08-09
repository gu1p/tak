/// Upper bound on the handler-error detail echoed into a 500 response body.
pub(super) const MAX_HANDLER_DETAIL_BYTES: usize = 512;

/// Renders a handler error into a single-line, bounded, control-char-free string
/// safe to place in an `ErrorResponse.message`. Keeps only the first line of the
/// `{err:#}` chain (the rest stays in the daemon log), collapses whitespace, and
/// truncates on a UTF-8 char boundary. Avoids log-injection and oversized bodies.
///
/// ```no_run
/// # // Reason: This private HTTP helper is exercised through server behavior tests.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn sanitize_handler_detail(err: &anyhow::Error) -> String {
    let raw = format!("{err:#}");
    let first_line = raw.lines().next().unwrap_or_default();
    let mut sanitized = String::with_capacity(first_line.len());
    let mut prev_was_space = false;
    for ch in first_line.chars() {
        let mapped = if ch.is_control() || ch.is_whitespace() {
            ' '
        } else {
            ch
        };
        if mapped == ' ' {
            if prev_was_space {
                continue;
            }
            prev_was_space = true;
        } else {
            prev_was_space = false;
        }
        sanitized.push(mapped);
    }
    let trimmed = sanitized.trim();
    if trimmed.len() <= MAX_HANDLER_DETAIL_BYTES {
        return trimmed.to_string();
    }
    // Reserve room for the ellipsis so the emitted detail never exceeds the cap.
    let ellipsis = "…";
    let mut end = MAX_HANDLER_DETAIL_BYTES - ellipsis.len();
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{ellipsis}", &trimmed[..end])
}
