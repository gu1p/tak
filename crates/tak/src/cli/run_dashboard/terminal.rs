use std::io::{self, Write};

use anyhow::{Context, Result};
use crossterm::{
    cursor, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::model::DashboardState;
use super::navigation::DashboardNavigation;

#[path = "terminal/cleanup.rs"]
mod cleanup;
#[path = "terminal/summary.rs"]
mod summary;

pub(super) use cleanup::{RawModeGuard, attempt_restore, restore_or_retain};
pub(super) use summary::final_summary;

type DashboardTerminal = Terminal<CrosstermBackend<io::Stderr>>;
type DashboardRawMode = RawModeGuard<fn() -> io::Result<()>>;

pub(super) struct TerminalDisplay {
    terminal: Option<DashboardTerminal>,
    color_enabled: bool,
    raw_mode: DashboardRawMode,
}

impl TerminalDisplay {
    pub(super) fn start(interactive: bool) -> Result<Self> {
        if interactive {
            enable_raw_mode().context("enable run dashboard raw mode")?;
        }
        let mut raw_mode =
            RawModeGuard::new(interactive, disable_raw_mode as fn() -> io::Result<()>);
        let mut stderr = io::stderr();
        if let Err(error) = execute!(stderr, EnterAlternateScreen) {
            let _ = attempt_restore(
                &mut stderr,
                |stderr| execute!(stderr, cursor::Show),
                |stderr| execute!(stderr, LeaveAlternateScreen),
                || raw_mode.restore(),
            );
            return Err(error).context("enter run dashboard screen");
        }
        if let Err(error) = execute!(stderr, cursor::Hide) {
            let _ = attempt_restore(
                &mut stderr,
                |stderr| execute!(stderr, cursor::Show),
                |stderr| execute!(stderr, LeaveAlternateScreen),
                || raw_mode.restore(),
            );
            return Err(error).context("hide terminal cursor");
        }
        let size = crossterm::terminal::size().unwrap_or((0, 0));
        let backend = CrosstermBackend::new(stderr);
        let created = if size.0 == 0 || size.1 == 0 {
            Terminal::with_options(
                backend,
                TerminalOptions {
                    viewport: Viewport::Fixed(Rect::new(0, 0, 120, 40)),
                },
            )
        } else {
            Terminal::new(backend)
        };
        let terminal = match created {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stderr = io::stderr();
                let _ = attempt_restore(
                    &mut stderr,
                    |stderr| execute!(stderr, cursor::Show),
                    |stderr| execute!(stderr, LeaveAlternateScreen),
                    || raw_mode.restore(),
                );
                return Err(error).context("create run dashboard terminal");
            }
        };
        Ok(Self {
            terminal: Some(terminal),
            color_enabled: std::env::var_os("NO_COLOR").is_none(),
            raw_mode,
        })
    }

    pub(super) fn draw(
        &mut self,
        state: &DashboardState,
        navigation: &DashboardNavigation,
    ) -> Result<()> {
        let Some(terminal) = self.terminal.as_mut() else {
            return Ok(());
        };
        terminal
            .draw(|frame| {
                super::render::draw_with_navigation(frame, state, navigation, self.color_enabled)
            })
            .context("draw run dashboard")?;
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        state: &DashboardState,
        navigation: &DashboardNavigation,
    ) -> Result<()> {
        self.draw(state, navigation)?;
        self.restore()?;
        let mut stderr = io::stderr().lock();
        writeln!(stderr, "{}", final_summary(state))?;
        stderr.flush()?;
        Ok(())
    }

    pub(super) fn release_raw_mode(&mut self) -> Result<()> {
        self.raw_mode.restore()
    }

    fn restore(&mut self) -> Result<()> {
        if self.terminal.is_none() {
            return self.raw_mode.restore();
        }
        let raw_mode = &mut self.raw_mode;
        restore_or_retain(&mut self.terminal, |terminal| {
            attempt_restore(
                terminal,
                |terminal| terminal.show_cursor(),
                |terminal| execute!(terminal.backend_mut(), LeaveAlternateScreen),
                || raw_mode.restore(),
            )
        })
    }
}

impl Drop for TerminalDisplay {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
