use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::RunEventKind;

use super::test_support::{event, state};

#[test]
fn task_output_cannot_inject_terminal_control_sequences() {
    let mut state = state();
    let mut output = event(1, RunEventKind::Stdout, "build", Some("worker-a"));
    output.chunk_base64 =
        Some(STANDARD.encode(b"before\x1b]52;c;Y2xpcGJvYXJk\x07after\r\nnext\x08"));

    state.apply(&output).unwrap();

    let text = &state.logs[0].text;
    assert!(text.contains("before") && text.contains("after") && text.contains("next"));
    assert!(
        text.chars()
            .all(|character| character == '\n' || !character.is_control()),
        "unsafe dashboard output: {text:?}"
    );
}

#[test]
fn failure_messages_cannot_inject_the_dashboard_or_final_summary() {
    let mut state = state();
    let mut failure = event(1, RunEventKind::Failed, "build", Some("worker-a"));
    failure.message = "failed\x1b[2J\x1b]52;c;Y2xpcGJvYXJk\x07 safely".into();

    state.apply(&failure).unwrap();

    let rendered = format!(
        "{}\n{}\n{}",
        state.error.as_deref().unwrap_or_default(),
        state.diagnostics.join("\n"),
        super::terminal::final_summary(&state)
    );
    assert!(rendered.contains("failed") && rendered.contains("safely"));
    assert!(
        rendered
            .chars()
            .all(|character| character == '\n' || !character.is_control()),
        "unsafe failure text: {rendered:?}"
    );
    assert!(state.logs.iter().all(|log| {
        log.text
            .chars()
            .all(|character| character == '\n' || !character.is_control())
    }));
}
