use std::io::{IsTerminal, stdout};

use anyhow::{Context, Result, bail};
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::app::RemoteAddApp;
use super::render::render;
use super::types::{AddAction, Screen};
use super::{CommandOutcome, handle_command};

pub(super) async fn run(mut app: RemoteAddApp) -> Result<()> {
    if !std::io::stdin().is_terminal() || !stdout().is_terminal() {
        bail!("remote add requires an interactive terminal or explicit token/--words arguments")
    }
    enable_raw_mode()?;
    let cleanup = TerminalCleanup::new();
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let outcome = loop {
        terminal.draw(|frame| render(frame, &app))?;
        let command = app.handle(read_action(app.screen)?)?;
        match handle_command(&mut app, command).await? {
            CommandOutcome::Continue => {}
            CommandOutcome::Saved(remote) => break Some(remote),
            CommandOutcome::Cancelled => break None,
        }
    };

    drop(terminal);
    cleanup.finish()?;
    if let Some(remote) = outcome {
        println!("added remote {}", remote.node_id);
    } else {
        println!("remote add cancelled");
    }
    Ok(())
}

fn read_action(screen: Screen) -> Result<AddAction> {
    loop {
        if let Event::Key(key) = event::read()?
            && let Some(action) = key_to_action(screen, key)
        {
            return Ok(action);
        }
    }
}

pub(super) fn key_to_action(screen: Screen, key: KeyEvent) -> Option<AddAction> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('u') | KeyCode::Char('U'))
    {
        return Some(AddAction::ClearInput);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
    {
        return Some(AddAction::Quit);
    }
    match key.code {
        KeyCode::Up => Some(AddAction::Up),
        KeyCode::Down => Some(AddAction::Down),
        KeyCode::Enter => Some(AddAction::Enter),
        KeyCode::Esc => Some(AddAction::Back),
        KeyCode::Backspace => Some(AddAction::Backspace),
        KeyCode::Char('q') if text_screen(screen) => Some(AddAction::Character('q')),
        KeyCode::Char('q') => Some(AddAction::Quit),
        KeyCode::Char(ch) => Some(AddAction::Character(ch)),
        _ => None,
    }
}

fn text_screen(screen: Screen) -> bool {
    matches!(screen, Screen::Words | Screen::Location)
}

struct CleanupOps {
    disable_raw_mode: fn() -> std::io::Result<()>,
    restore_screen: fn() -> std::io::Result<()>,
}

struct TerminalCleanup {
    ops: CleanupOps,
    armed: bool,
}

impl TerminalCleanup {
    fn new() -> Self {
        Self {
            ops: CleanupOps {
                disable_raw_mode,
                restore_screen,
            },
            armed: true,
        }
    }

    fn finish(mut self) -> Result<()> {
        self.armed = false;
        self.run_cleanup()
    }

    fn run_cleanup(&self) -> Result<()> {
        let disable_err = (self.ops.disable_raw_mode)()
            .context("failed to disable raw mode")
            .err();
        let restore_err = (self.ops.restore_screen)()
            .context("failed to restore terminal screen")
            .err();

        if let Some(err) = disable_err {
            return Err(err);
        }
        if let Some(err) = restore_err {
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if self.armed {
            self.armed = false;
            let _ = self.run_cleanup();
        }
    }
}

fn restore_screen() -> std::io::Result<()> {
    let mut out = stdout();
    execute!(out, LeaveAlternateScreen, cursor::Show)
}
