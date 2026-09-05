use super::{width, wrap};

#[test]
fn wrapping_at_a_word_boundary_does_not_invent_indentation() {
    assert_eq!(wrap("one two three", 7), ["one two", "three"]);
}

#[test]
fn wide_unicode_uses_terminal_cells_and_preserves_every_character() {
    let text = "编译任务失败";
    let wrapped = wrap(text, 5);
    assert!(wrapped.iter().all(|line| width(line) <= 5));
    assert_eq!(wrapped.concat(), text);
}

#[test]
fn source_indentation_and_explicit_empty_lines_are_preserved() {
    assert_eq!(
        wrap("    let value = 1;\n\nnext", 40),
        ["    let value = 1;", "", "next"]
    );
}
