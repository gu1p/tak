use super::super::framing::LineFramer;

#[test]
fn complete_lines_wait_for_fragments_and_preserve_invalid_utf8() {
    let mut framer = LineFramer::new(16 * 1024);

    assert!(framer.push(b"part").is_empty());
    assert_eq!(framer.push(b"ial\nnext"), vec![b"partial\n".to_vec()]);
    assert_eq!(
        framer.push(&[0xff, b'\n']),
        vec![vec![b'n', b'e', b'x', b't', 0xff, b'\n']]
    );
    assert!(framer.finish().is_empty());
}

#[test]
fn trailing_and_oversized_fragments_flush_readably() {
    let mut framer = LineFramer::new(4);

    assert_eq!(framer.push(b"abcde"), vec![b"abcd".to_vec()]);
    assert_eq!(framer.finish(), vec![b"e".to_vec()]);
}
