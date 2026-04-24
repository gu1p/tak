use anyhow::{Result, bail};
use tak_proto::{TOR_INVITE_WORD_COUNT, decode_tor_invite_words, encode_tor_invite};

use super::super::remote_inventory::RemoteRecord;
use super::types::{AddAction, AppCommand, Method, Screen, StartMode};

pub(super) struct RemoteAddApp {
    pub(super) screen: Screen,
    pub(super) selected_method: Method,
    pub(super) words: Vec<String>,
    pub(super) word_input: String,
    pub(super) location_input: String,
    pub(super) remote: Option<RemoteRecord>,
    pub(super) message: Option<String>,
}

impl RemoteAddApp {
    pub(super) fn new(start: StartMode) -> Self {
        let screen = match start {
            StartMode::Menu => Screen::Method,
            StartMode::Words => Screen::Words,
        };
        Self {
            screen,
            selected_method: Method::Words,
            words: Vec::new(),
            word_input: String::new(),
            location_input: String::new(),
            remote: None,
            message: None,
        }
    }

    pub(super) fn handle(&mut self, action: AddAction) -> Result<AppCommand> {
        if matches!(action, AddAction::Quit) {
            return Ok(AppCommand::Cancel);
        }
        self.message = None;
        match self.screen {
            Screen::Method => self.handle_method(action),
            Screen::Words => self.handle_words(action),
            Screen::Location => self.handle_location(action),
            Screen::Confirm => self.handle_confirm(action),
        }
    }

    pub(super) fn show_remote(&mut self, remote: RemoteRecord) {
        self.remote = Some(remote);
        self.screen = Screen::Confirm;
        self.message = Some("Remote reached. Review before saving.".to_string());
    }

    pub(super) fn show_error(&mut self, message: String) {
        self.message = Some(message);
    }

    fn handle_method(&mut self, action: AddAction) -> Result<AppCommand> {
        match action {
            AddAction::Up | AddAction::Down => self.toggle_method(),
            AddAction::Enter => self.open_selected_method(),
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn toggle_method(&mut self) {
        self.selected_method = match self.selected_method {
            Method::Words => Method::Location,
            Method::Location => Method::Words,
        };
    }

    fn open_selected_method(&mut self) {
        self.screen = match self.selected_method {
            Method::Words => Screen::Words,
            Method::Location => Screen::Location,
        };
    }

    fn handle_words(&mut self, action: AddAction) -> Result<AppCommand> {
        match action {
            AddAction::Back if self.words.is_empty() && self.word_input.is_empty() => {
                self.screen = Screen::Method;
            }
            AddAction::Back | AddAction::UndoWord => self.undo_word(),
            AddAction::Backspace => self.backspace_word_input(),
            AddAction::ClearInput => self.word_input.clear(),
            AddAction::Enter => return self.commit_word_input(),
            AddAction::Character(ch) if ch.is_whitespace() => return self.commit_word_input(),
            AddAction::Character(ch) => self.word_input.push(ch),
            AddAction::Paste(value) => return self.add_words_from_text(&value),
            AddAction::Word(value) => return self.add_words_from_text(&value),
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn handle_location(&mut self, action: AddAction) -> Result<AppCommand> {
        match action {
            AddAction::Back if self.location_input.is_empty() => self.screen = Screen::Method,
            AddAction::Back | AddAction::Backspace => {
                self.location_input.pop();
            }
            AddAction::ClearInput => self.location_input.clear(),
            AddAction::Enter => return self.location_probe_command(),
            AddAction::Character(ch) => self.location_input.push(ch),
            AddAction::Paste(value) => self.location_input.push_str(&value),
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn handle_confirm(&mut self, action: AddAction) -> Result<AppCommand> {
        match action {
            AddAction::Enter => {
                let remote = self
                    .remote
                    .clone()
                    .expect("confirmation screen requires a remote");
                Ok(AppCommand::Save(remote))
            }
            AddAction::Back => Ok(AppCommand::Cancel),
            _ => Ok(AppCommand::Continue),
        }
    }

    fn backspace_word_input(&mut self) {
        if self.word_input.pop().is_none() {
            self.undo_word();
        }
    }

    fn undo_word(&mut self) {
        if self.words.pop().is_some() {
            self.message = Some(format!("Removed word {:02}", self.words.len() + 1));
        }
    }

    fn commit_word_input(&mut self) -> Result<AppCommand> {
        if self.word_input.trim().is_empty() {
            return self.maybe_probe_words();
        }
        let word = std::mem::take(&mut self.word_input);
        self.add_words_from_text(&word)
    }

    fn add_words_from_text(&mut self, text: &str) -> Result<AppCommand> {
        for value in text.split_whitespace() {
            if self.words.len() == TOR_INVITE_WORD_COUNT {
                self.message = Some("All 19 words are already filled.".to_string());
                break;
            }
            match tak_proto::normalize_tor_invite_word(value) {
                Ok(word) => self.words.push(word),
                Err(err) => {
                    self.message = Some(err.to_string());
                    break;
                }
            }
        }
        self.maybe_probe_words()
    }

    fn maybe_probe_words(&mut self) -> Result<AppCommand> {
        if self.words.len() != TOR_INVITE_WORD_COUNT {
            return Ok(AppCommand::Continue);
        }
        let phrase = self.words.join(" ");
        match decode_tor_invite_words(&phrase) {
            Ok(token) => Ok(AppCommand::Probe(token)),
            Err(err) => {
                self.message = Some(err.to_string());
                Ok(AppCommand::Continue)
            }
        }
    }

    fn location_probe_command(&mut self) -> Result<AppCommand> {
        match token_from_location_input(&self.location_input) {
            Ok(token) => Ok(AppCommand::Probe(token)),
            Err(err) => {
                self.message = Some(err.to_string());
                Ok(AppCommand::Continue)
            }
        }
    }
}

pub(super) fn token_from_location_input(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.starts_with("takd:v1:") || trimmed.starts_with("takd:tor:") {
        return Ok(trimmed.to_string());
    }
    if trimmed.contains(".onion") {
        return encode_tor_invite(trimmed);
    }
    bail!("paste a takd token, takd tor invite, or Tor .onion location");
}
