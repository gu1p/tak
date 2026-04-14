use std::io::{IsTerminal, stdout};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::app::{AppAction, AppCommand, ScanApp};
use super::provider::CameraCatalog;
use super::render::render;
use crate::cli::remote_inventory::add_remote;

pub(super) async fn run(mut app: ScanApp, catalog: &dyn CameraCatalog) -> Result<()> {
    if !std::io::stdin().is_terminal() || !stdout().is_terminal() {
        bail!("remote scan requires an interactive terminal")
    }
    enable_raw_mode()?;
    let cleanup = TerminalCleanup::new();
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let outcome = loop {
        terminal.draw(|frame| render(frame, &app))?;
        let action = read_action(app.screen == super::app::Screen::Preview)?;
        match app.handle(action, catalog)? {
            AppCommand::Continue => {}
            AppCommand::Quit => break None,
            AppCommand::AddToken(token) => break Some(add_remote(&token).await?),
        }
    };

    drop(terminal);
    cleanup.finish()?;
    if let Some(remote) = outcome {
        println!("added remote {}", remote.node_id);
    }
    Ok(())
}

fn read_action(preview_mode: bool) -> Result<AppAction> {
    if preview_mode && !event::poll(Duration::from_millis(120))? {
        return Ok(AppAction::Tick);
    }
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            return Ok(match key.code {
                KeyCode::Up => AppAction::Up,
                KeyCode::Down => AppAction::Down,
                KeyCode::Enter => AppAction::Enter,
                KeyCode::Esc => AppAction::Back,
                KeyCode::Char('q') => AppAction::Quit,
                _ if preview_mode => AppAction::Tick,
                _ => continue,
            });
        }
        if preview_mode {
            return Ok(AppAction::Tick);
        }
    }
}

pub(super) struct CleanupOps {
    pub(super) disable_raw_mode: fn() -> std::io::Result<()>,
    pub(super) restore_screen: fn() -> std::io::Result<()>,
}

pub(super) struct TerminalCleanup {
    ops: CleanupOps,
    armed: bool,
}

impl TerminalCleanup {
    fn new() -> Self {
        Self::with_ops(CleanupOps {
            disable_raw_mode,
            restore_screen,
        })
    }

    pub(super) fn with_ops(ops: CleanupOps) -> Self {
        Self { ops, armed: true }
    }

    pub(super) fn finish(mut self) -> Result<()> {
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
