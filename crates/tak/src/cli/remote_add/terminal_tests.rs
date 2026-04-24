use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::terminal::key_to_action;
use super::types::{AddAction, Screen};

#[test]
fn lowercase_q_is_text_on_input_screens() {
    assert!(matches!(
        key_to_action(Screen::Words, key('q')),
        Some(AddAction::Character('q'))
    ));
    assert!(matches!(
        key_to_action(Screen::Location, key('q')),
        Some(AddAction::Character('q'))
    ));
}

#[test]
fn lowercase_q_still_quits_non_input_screens() {
    assert!(matches!(
        key_to_action(Screen::Method, key('q')),
        Some(AddAction::Quit)
    ));
    assert!(matches!(
        key_to_action(Screen::Confirm, key('q')),
        Some(AddAction::Quit)
    ));
}

fn key(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
}
