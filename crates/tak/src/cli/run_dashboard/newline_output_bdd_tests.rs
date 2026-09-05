use base64::Engine;
use tak_proto::local_daemon::v2::RunEventKind;

use super::render_test_support::frame_at_size;
use super::test_support::{event, state};

#[test]
fn normal_line_endings_do_not_insert_blank_log_rows() {
    let mut state = state();
    for index in 0..12 {
        let mut output = event(index + 1, RunEventKind::Stdout, "build", Some("worker-a"));
        output.chunk_base64 =
            Some(base64::engine::general_purpose::STANDARD.encode(format!("output-{index:02}\n")));
        state.apply(&output).unwrap();
    }
    let rendered = frame_at_size(&state, 100, 24);
    assert!(
        rendered.contains("output-06") && rendered.contains("output-11"),
        "{rendered}"
    );
}

#[test]
fn an_explicit_blank_output_line_is_preserved_once() {
    let mut state = state();
    let mut output = event(1, RunEventKind::Stdout, "build", Some("worker-a"));
    output.chunk_base64 = Some(base64::engine::general_purpose::STANDARD.encode("first\n\n"));
    state.apply(&output).unwrap();
    let rendered = frame_at_size(&state, 100, 24);
    assert_eq!(
        rendered.matches("//:build@worker-a │").count(),
        2,
        "{rendered}"
    );
}
