use std::io::{self, IsTerminal, Write};

use anyhow::Result;
use crossterm::{
    cursor::MoveUp,
    execute,
    terminal::{Clear, ClearType},
};

use super::model::RunState;
use super::render::{render_frame, render_plain};

pub(super) struct TerminalDisplay {
    inline: bool,
    color: bool,
    drawn_lines: usize,
}

impl TerminalDisplay {
    pub(super) fn detect() -> Self {
        let inline = io::stderr().is_terminal();
        Self {
            inline,
            color: inline && std::env::var_os("NO_COLOR").is_none(),
            drawn_lines: 0,
        }
    }

    pub(super) fn is_inline(&self) -> bool {
        self.inline
    }

    pub(super) fn begin_log(&mut self) -> Result<()> {
        if !self.inline || self.drawn_lines == 0 {
            return Ok(());
        }
        let mut stderr = io::stderr().lock();
        execute!(
            stderr,
            MoveUp(u16::try_from(self.drawn_lines).unwrap_or(u16::MAX)),
            Clear(ClearType::FromCursorDown),
        )?;
        self.drawn_lines = 0;
        Ok(())
    }

    pub(super) fn redraw(&mut self, state: &RunState) -> Result<()> {
        if !self.inline {
            return Ok(());
        }
        self.write_frame(render_frame(state, terminal_width(), self.color))
    }

    pub(super) fn final_frame(&mut self, state: &RunState) -> Result<()> {
        if self.inline {
            return self.redraw(state);
        }
        self.write_frame(render_plain(state, terminal_width()))
    }

    fn write_frame(&mut self, frame: String) -> Result<()> {
        let mut stderr = io::stderr().lock();
        stderr.write_all(frame.as_bytes())?;
        stderr.flush()?;
        self.drawn_lines = displayed_line_count(&frame, terminal_width());
        Ok(())
    }
}

pub(super) fn displayed_line_count(frame: &str, width: usize) -> usize {
    let width = width.max(1);
    frame
        .lines()
        .map(|line| {
            let visible = visible_character_count(line);
            visible.max(1).div_ceil(width)
        })
        .sum()
}

fn visible_character_count(line: &str) -> usize {
    let mut escape = false;
    line.chars()
        .filter(|character| {
            if escape {
                if *character == 'm' {
                    escape = false;
                }
                return false;
            }
            if *character == '\u{1b}' {
                escape = true;
                return false;
            }
            true
        })
        .count()
}

fn terminal_width() -> usize {
    crossterm::terminal::size().map_or(100, |(width, _)| usize::from(width))
}
