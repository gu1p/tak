use std::io;

use anyhow::{Context, Result};

pub(in crate::cli::run_dashboard) fn restore_or_retain<T>(
    target: &mut Option<T>,
    restore: impl FnOnce(&mut T) -> Result<()>,
) -> Result<()> {
    let Some(mut target_value) = target.take() else {
        return Ok(());
    };
    match restore(&mut target_value) {
        Ok(()) => Ok(()),
        Err(error) => {
            *target = Some(target_value);
            Err(error)
        }
    }
}

pub(in crate::cli::run_dashboard) fn attempt_restore<T, ShowCursor, LeaveScreen, DisableRaw>(
    target: &mut T,
    show_cursor: ShowCursor,
    leave_screen: LeaveScreen,
    disable_raw: DisableRaw,
) -> Result<()>
where
    ShowCursor: FnOnce(&mut T) -> io::Result<()>,
    LeaveScreen: FnOnce(&mut T) -> io::Result<()>,
    DisableRaw: FnOnce() -> Result<()>,
{
    let cursor_result = show_cursor(target).context("show terminal cursor");
    let screen_result = leave_screen(target).context("leave run dashboard screen");
    let raw_result = disable_raw().context("disable run dashboard raw mode");
    cursor_result.and(screen_result).and(raw_result)
}

pub(in crate::cli::run_dashboard) struct RawModeGuard<DisableRaw>
where
    DisableRaw: FnMut() -> io::Result<()>,
{
    enabled: bool,
    disable_raw: DisableRaw,
}

impl<DisableRaw> RawModeGuard<DisableRaw>
where
    DisableRaw: FnMut() -> io::Result<()>,
{
    pub(in crate::cli::run_dashboard) fn new(enabled: bool, disable_raw: DisableRaw) -> Self {
        Self {
            enabled,
            disable_raw,
        }
    }

    pub(in crate::cli::run_dashboard) fn restore(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        (self.disable_raw)().context("disable run dashboard raw mode")?;
        self.enabled = false;
        Ok(())
    }
}

impl<DisableRaw> Drop for RawModeGuard<DisableRaw>
where
    DisableRaw: FnMut() -> io::Result<()>,
{
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
