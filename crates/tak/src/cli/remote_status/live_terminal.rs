use std::io::stdout;

use anyhow::{Context, Result};
use crossterm::cursor;
use crossterm::execute;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::Terminal;
use ratatui::backend::Backend;

pub(super) struct TerminalCleanup {
    armed: bool,
}

impl TerminalCleanup {
    pub(super) fn new() -> Self {
        Self { armed: true }
    }

    fn finish(mut self) -> Result<()> {
        self.armed = false;
        restore_screen().context("restore remote status terminal")
    }
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if self.armed {
            self.armed = false;
            let _ = restore_screen();
        }
    }
}

pub(super) fn finish_terminal<B: Backend>(
    terminal: Terminal<B>,
    cleanup: Option<TerminalCleanup>,
) -> Result<()> {
    drop(terminal);
    if let Some(cleanup) = cleanup {
        cleanup.finish()?;
    }
    Ok(())
}

fn restore_screen() -> std::io::Result<()> {
    let mut out = stdout();
    execute!(out, LeaveAlternateScreen, cursor::Show)
}
