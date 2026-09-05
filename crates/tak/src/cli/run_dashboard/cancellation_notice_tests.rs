use super::render_test_support::frame;
use super::terminal::final_summary;
use super::test_support::state;

#[test]
fn persisted_cancellation_is_visible_inside_the_dashboard_and_after_restoration() {
    let mut state = state();
    state.note_cancellation_persisted();

    let rendered = frame(&state, 118);
    let summary = final_summary(&state);

    assert!(rendered.contains("CANCELLING"), "{rendered}");
    assert!(
        rendered.contains("Cancellation persisted · waiting for takd to stop active work"),
        "{rendered}"
    );
    assert!(summary.contains("Cancellation persisted"), "{summary}");
}

#[test]
fn already_terminal_cancellation_race_has_a_visible_dashboard_acknowledgement() {
    let mut state = state();
    state.note_already_terminal();

    let rendered = frame(&state, 118);

    assert!(
        rendered.contains("Run was already terminal · loading its final state"),
        "{rendered}"
    );
}
