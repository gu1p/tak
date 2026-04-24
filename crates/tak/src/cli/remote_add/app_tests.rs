use tak_proto::{decode_tor_invite, encode_tor_invite, encode_tor_invite_words};

use super::app::{RemoteAddApp, token_from_location_input};
use super::types::{AddAction, AppCommand, Screen, StartMode};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn location_input_accepts_existing_tokens_and_onion_locations() {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    assert_eq!(
        token_from_location_input(&invite).expect("existing invite"),
        invite
    );

    let token = token_from_location_input(V3_BASE_URL).expect("onion location");
    assert_eq!(
        decode_tor_invite(&token).expect("decode invite"),
        V3_BASE_URL
    );
    assert!(token_from_location_input("http://127.0.0.1:3000").is_err());
}

#[test]
fn words_screen_can_undo_previous_words_before_decoding() {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    let words = encode_tor_invite_words(&invite)
        .expect("encode words")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut app = RemoteAddApp::new(StartMode::Words);

    assert!(matches!(
        app.handle(AddAction::Word(words[0].clone()))
            .expect("first word"),
        AppCommand::Continue
    ));
    assert!(matches!(
        app.handle(AddAction::Word(words[1].clone()))
            .expect("second word"),
        AppCommand::Continue
    ));
    assert_eq!(app.words.len(), 2);

    app.handle(AddAction::UndoWord).expect("undo");
    assert_eq!(app.words, vec![words[0].clone()]);
    assert_eq!(app.message.as_deref(), Some("Removed word 02"));
}

#[test]
fn location_screen_keeps_invalid_input_inside_tui() {
    let mut app = RemoteAddApp::new(StartMode::Menu);

    app.handle(AddAction::Down).expect("select location");
    app.handle(AddAction::Enter).expect("open location");
    app.handle(AddAction::Paste("http://127.0.0.1:3000".to_string()))
        .expect("paste invalid location");

    assert!(matches!(
        app.handle(AddAction::Enter).expect("validate location"),
        AppCommand::Continue
    ));
    assert!(matches!(app.screen, Screen::Location));
    assert_eq!(app.location_input, "http://127.0.0.1:3000");
    assert_eq!(
        app.message.as_deref(),
        Some("paste a takd token, takd tor invite, or Tor .onion location")
    );
}
