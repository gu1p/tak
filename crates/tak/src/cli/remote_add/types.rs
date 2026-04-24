use super::super::remote_inventory::RemoteRecord;

#[derive(Clone, Copy)]
pub(crate) enum StartMode {
    Menu,
    Words,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    Method,
    Words,
    Location,
    Confirm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    Words,
    Location,
}

#[derive(Clone)]
pub(super) enum AddAction {
    Up,
    Down,
    Enter,
    Back,
    Quit,
    Backspace,
    ClearInput,
    Character(char),
    Paste(String),
    Word(String),
    UndoWord,
}

pub(super) enum AppCommand {
    Continue,
    Cancel,
    Probe(String),
    Save(RemoteRecord),
}
