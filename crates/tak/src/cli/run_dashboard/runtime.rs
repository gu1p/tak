use std::io::{self, IsTerminal};

use anyhow::Result;
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind, RunLifecycleState};

use super::input::{DashboardInput, InputAction};
use super::model::{DashboardSeed, DashboardState};
use super::navigation::DashboardNavigation;
use super::terminal::TerminalDisplay;

pub(in crate::cli) struct RunDashboard {
    state: DashboardState,
    display: TerminalDisplay,
    input: DashboardInput,
    navigation: DashboardNavigation,
    capture_stdout: bool,
}

impl RunDashboard {
    pub(in crate::cli) fn wanted() -> bool {
        io::stderr().is_terminal()
    }

    pub(in crate::cli) fn detect(seed: DashboardSeed) -> Result<Option<Self>> {
        if !Self::wanted() {
            return Ok(None);
        }
        let interactive = io::stdin().is_terminal();
        let mut dashboard = Self {
            state: DashboardState::new(seed),
            display: TerminalDisplay::start(interactive)?,
            input: DashboardInput::new(interactive),
            navigation: DashboardNavigation::default(),
            capture_stdout: io::stdout().is_terminal(),
        };
        dashboard.draw()?;
        Ok(Some(dashboard))
    }

    pub(in crate::cli) fn render_event(&mut self, event: &RunEvent) -> Result<bool> {
        self.state.apply(event)?;
        self.draw()?;
        Ok(match event.kind {
            RunEventKind::Stdout => self.capture_stdout,
            RunEventKind::Stderr => true,
            _ => true,
        })
    }

    pub(in crate::cli) fn refresh(&mut self) -> Result<()> {
        self.draw()
    }

    pub(in crate::cli) fn render_page_state(&mut self, lifecycle: RunLifecycleState) -> Result<()> {
        self.state.sync_lifecycle(lifecycle);
        self.draw()
    }

    pub(in crate::cli) async fn next_interrupt(&mut self) -> Result<()> {
        loop {
            match self.input.next().await? {
                InputAction::Navigate(action) => {
                    self.navigation.apply(action);
                    self.draw()?;
                }
                InputAction::Redraw => self.draw()?,
                InputAction::Interrupt => return Ok(()),
                InputAction::InputLost => {
                    self.display.release_raw_mode()?;
                    self.state.note_input_lost();
                    self.draw()?;
                }
            }
        }
    }

    pub(in crate::cli) fn note_cancellation_persisted(&mut self) -> Result<()> {
        self.state.note_cancellation_persisted();
        self.draw()
    }

    pub(in crate::cli) fn note_already_terminal(&mut self) -> Result<()> {
        self.state.note_already_terminal();
        self.draw()
    }

    pub(in crate::cli) fn note_logs_expired(&mut self) -> Result<()> {
        self.state.note_logs_expired();
        self.draw()
    }

    pub(in crate::cli) fn finish(
        &mut self,
        lifecycle: RunLifecycleState,
        error: Option<String>,
    ) -> Result<()> {
        self.state.lifecycle = lifecycle.as_str().into();
        if error.is_some() {
            self.state.error = error;
        }
        self.display.finish(&self.state, &self.navigation)
    }

    fn draw(&mut self) -> Result<()> {
        self.display.draw(&self.state, &self.navigation)
    }
}
