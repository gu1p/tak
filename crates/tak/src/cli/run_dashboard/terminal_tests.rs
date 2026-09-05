use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};

use super::terminal::final_summary;
use super::test_support::{event, state};

#[test]
fn successful_summary_preserves_only_the_recent_log_tail_in_scrollback() {
    let mut state = state();
    for sequence in 1..=12 {
        let mut output = event(sequence, RunEventKind::Stdout, "build", Some("worker-a"));
        output.chunk_base64 = Some(STANDARD.encode(format!("log-{sequence:02}\n")));
        state.apply(&output).unwrap();
    }
    state.sync_lifecycle(RunLifecycleState::Succeeded);

    let summary = final_summary(&state);
    let output_lines = summary
        .lines()
        .filter(|line| line.contains(" │ log-"))
        .collect::<Vec<_>>();

    assert_eq!(output_lines.len(), 8, "{summary}");
    assert!(!summary.contains("log-04"), "{summary}");
    assert!(
        summary.contains("log-05") && summary.contains("log-12"),
        "{summary}"
    );
}
