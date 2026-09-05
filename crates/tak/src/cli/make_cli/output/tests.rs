use base64::Engine as _;
use tak_core::model::TaskLabel;
use tak_exec::OutputStream;
use tak_make::ParallelOutputMode;
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::{OutputVisibility, ParallelMakeOutputObserver};
use crate::cli::daemon_run::PersistedEventRenderer;
use crate::cli::make_cli::task::ParallelMakeGoal;

#[test]
fn dashboard_capture_suppresses_only_streams_owned_by_the_terminal_ui() {
    let interactive = OutputVisibility::for_attachment(true, true);
    assert!(!interactive.writes(OutputStream::Stdout));
    assert!(!interactive.writes(OutputStream::Stderr));

    let redirected_stdout = OutputVisibility::for_attachment(true, false);
    assert!(redirected_stdout.writes(OutputStream::Stdout));
    assert!(!redirected_stdout.writes(OutputStream::Stderr));

    let redirected = OutputVisibility::for_attachment(false, false);
    assert!(redirected.writes(OutputStream::Stdout));
    assert!(redirected.writes(OutputStream::Stderr));
}

#[test]
fn suppressed_dashboard_output_still_records_the_original_make_failure() {
    let label = TaskLabel {
        package: "//".into(),
        name: "make-0".into(),
    };
    let goals = vec![ParallelMakeGoal {
        label: label.clone(),
        goal: "check".into(),
        output: ParallelOutputMode::Grouped,
    }];
    let observer = ParallelMakeOutputObserver::with_visibility(
        &goals,
        None,
        OutputVisibility::for_attachment(true, true),
    );
    observer
        .render(&output_event(b"make: *** [check] Error 23\n"))
        .unwrap();
    observer.render(&failed_event()).unwrap();

    assert_eq!(observer.first_failure(&goals).unwrap(), Some(23));
    assert!(observer.state.lock().unwrap().grouped.is_empty());
}

#[test]
fn persisted_renderer_tracks_dashboard_activation_and_fallback() {
    let observer = ParallelMakeOutputObserver::with_visibility(
        &[],
        None,
        OutputVisibility::for_attachment(false, true),
    );
    assert!(observer.visibility.writes(OutputStream::Stderr));

    <ParallelMakeOutputObserver as PersistedEventRenderer>::set_dashboard_active(&observer, true);
    assert!(!observer.visibility.writes(OutputStream::Stderr));

    <ParallelMakeOutputObserver as PersistedEventRenderer>::set_dashboard_active(&observer, false);
    assert!(observer.visibility.writes(OutputStream::Stderr));
}

fn output_event(bytes: &[u8]) -> RunEvent {
    event(
        RunEventKind::Stderr,
        Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
    )
}

fn failed_event() -> RunEvent {
    let mut event = event(RunEventKind::Failed, None);
    event.exit_code = Some(2);
    event
}

fn event(kind: RunEventKind, chunk_base64: Option<String>) -> RunEvent {
    RunEvent {
        seq: 1,
        kind,
        job_id: Some("job-0".into()),
        task_ids: vec!["//:make-0".into()],
        node_id: Some("local".into()),
        authored_attempt: None,
        message: String::new(),
        chunk_base64,
        exit_code: None,
    }
}
