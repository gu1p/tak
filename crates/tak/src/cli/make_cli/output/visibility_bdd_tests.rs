use tak_exec::OutputStream;

use super::visibility::OutputVisibility;

#[test]
fn output_follows_the_dashboard_that_is_actually_available() {
    let visibility = OutputVisibility::for_attachment(false, true);
    assert!(visibility.writes(OutputStream::Stdout));
    assert!(visibility.writes(OutputStream::Stderr));

    visibility.set_dashboard_active(true);
    assert!(!visibility.writes(OutputStream::Stdout));
    assert!(!visibility.writes(OutputStream::Stderr));

    visibility.set_dashboard_active(false);
    assert!(visibility.writes(OutputStream::Stdout));
    assert!(visibility.writes(OutputStream::Stderr));
}
