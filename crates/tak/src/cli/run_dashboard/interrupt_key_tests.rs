use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::navigation::{NavigationAction, key_to_action};

#[test]
fn ctrl_c_interrupt_requires_a_press_event() {
    let ctrl_c = |kind| KeyEvent::new_with_kind(KeyCode::Char('c'), KeyModifiers::CONTROL, kind);

    assert_eq!(
        key_to_action(ctrl_c(KeyEventKind::Press)),
        Some(NavigationAction::Interrupt)
    );
    assert_eq!(key_to_action(ctrl_c(KeyEventKind::Repeat)), None);
    assert_eq!(key_to_action(ctrl_c(KeyEventKind::Release)), None);

    let repeated_down =
        KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat);
    assert_eq!(
        key_to_action(repeated_down),
        Some(NavigationAction::LineDown)
    );
}
