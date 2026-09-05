use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::navigation::key_to_action;

#[test]
fn holding_ctrl_c_does_not_count_as_the_second_detach_request() {
    let repeated = KeyEvent::new_with_kind(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
        KeyEventKind::Repeat,
    );

    assert_eq!(key_to_action(repeated), None);
}
