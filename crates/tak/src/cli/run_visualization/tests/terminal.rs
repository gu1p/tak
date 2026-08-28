use super::super::terminal::displayed_line_count;

#[test]
fn inline_frame_height_counts_wrapped_columns_without_counting_ansi() {
    assert_eq!(displayed_line_count("\u{1b}[31m123456\u{1b}[0m\n", 4), 2);
    assert_eq!(displayed_line_count("title\n\nrow\n", 80), 3);
}
