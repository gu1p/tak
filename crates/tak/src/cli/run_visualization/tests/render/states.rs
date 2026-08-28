use super::super::super::model::RunState;
use super::super::super::render::render_plain;

#[test]
fn loading_frame_is_explicit_before_the_plan_arrives() {
    let rendered = render_plain(&RunState::new(4), 100);
    assert!(rendered.contains("planning"), "{rendered}");
    assert!(rendered.contains("Planning execution graph"), "{rendered}");
}

#[test]
fn completed_empty_and_error_frames_explain_what_happened() {
    let mut empty = RunState::new(2);
    empty.finish(None);
    let empty_frame = render_plain(&empty, 100);
    assert!(
        empty_frame.contains("No executable task steps"),
        "{empty_frame}"
    );

    let mut failed = RunState::new(2);
    failed.finish(Some("workspace upload failed".into()));
    let failed_frame = render_plain(&failed, 100);
    assert!(failed_frame.contains("Run failed"), "{failed_frame}");
    assert!(
        failed_frame.contains("workspace upload failed"),
        "{failed_frame}"
    );
}
