use std::io;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::input::{InputAction, StreamDisposition, classify_stream_item, next_stream_action};
use super::navigation::NavigationAction;

#[test]
fn terminal_input_eof_and_read_errors_disable_navigation_without_failing_the_run() {
    assert!(matches!(
        classify_stream_item(None),
        StreamDisposition::Disable
    ));
    assert!(matches!(
        classify_stream_item(Some(Err(io::Error::other("terminal disappeared")))),
        StreamDisposition::Disable
    ));
}

#[test]
fn valid_terminal_input_remains_actionable() {
    let item = Some(Ok(Event::Key(KeyEvent::new(
        KeyCode::End,
        KeyModifiers::NONE,
    ))));
    assert!(matches!(
        classify_stream_item(item),
        StreamDisposition::Action(super::input::InputAction::Navigate(NavigationAction::End))
    ));
}

#[tokio::test]
async fn stream_error_becomes_a_runtime_visible_input_loss_and_disables_the_reader() {
    let mut events = Some(futures::stream::iter([Err(io::Error::other(
        "terminal disappeared",
    ))]));

    assert!(matches!(
        next_stream_action(&mut events).await.unwrap(),
        InputAction::InputLost
    ));
    assert!(events.is_none());
}
