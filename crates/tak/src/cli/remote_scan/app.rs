use anyhow::Result;

use super::decode::{ScanMatch, decode_frame};
use super::provider::{CameraCatalog, CameraDescriptor, CameraSession, GrayFrame};

#[derive(Clone, Copy)]
pub(super) enum AppAction {
    Up,
    Down,
    Enter,
    Back,
    Quit,
    Tick,
}

pub(super) enum AppCommand {
    Continue,
    Quit,
    AddToken(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    Picker,
    Preview,
    Confirm,
}

pub(super) struct ScanApp {
    pub(super) screen: Screen,
    pub(super) cameras: Vec<CameraDescriptor>,
    pub(super) selected: usize,
    pub(super) preview: Option<GrayFrame>,
    pub(super) detected: Option<ScanMatch>,
    session: Option<Box<dyn CameraSession>>,
}

impl ScanApp {
    pub(super) fn new(cameras: Vec<CameraDescriptor>) -> Self {
        Self {
            screen: Screen::Picker,
            cameras,
            selected: 0,
            preview: None,
            detected: None,
            session: None,
        }
    }

    pub(super) fn handle(
        &mut self,
        action: AppAction,
        catalog: &dyn CameraCatalog,
    ) -> Result<AppCommand> {
        if matches!(action, AppAction::Quit) {
            return Ok(AppCommand::Quit);
        }
        match self.screen {
            Screen::Picker => self.handle_picker(action, catalog),
            Screen::Preview => self.handle_preview(action),
            Screen::Confirm => self.handle_confirm(action),
        }
    }

    fn handle_picker(
        &mut self,
        action: AppAction,
        catalog: &dyn CameraCatalog,
    ) -> Result<AppCommand> {
        match action {
            AppAction::Up if self.selected > 0 => self.selected -= 1,
            AppAction::Down if self.selected + 1 < self.cameras.len() => self.selected += 1,
            AppAction::Enter => {
                self.session = Some(catalog.open(self.selected)?);
                self.preview = None;
                self.detected = None;
                self.screen = Screen::Preview;
            }
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn handle_preview(&mut self, action: AppAction) -> Result<AppCommand> {
        match action {
            AppAction::Back => {
                self.session = None;
                self.preview = None;
                self.detected = None;
                self.screen = Screen::Picker;
            }
            AppAction::Tick => self.capture_preview()?,
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn handle_confirm(&mut self, action: AppAction) -> Result<AppCommand> {
        match action {
            AppAction::Back => {
                self.detected = None;
                self.screen = Screen::Preview;
            }
            AppAction::Enter => {
                if let Some(found) = &self.detected {
                    return Ok(AppCommand::AddToken(found.token.clone()));
                }
            }
            _ => {}
        }
        Ok(AppCommand::Continue)
    }

    fn capture_preview(&mut self) -> Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        let frame = session.next_frame()?;
        let detected = decode_frame(&frame)?;
        self.preview = Some(frame);
        if let Some(found) = detected {
            self.detected = Some(found);
            self.screen = Screen::Confirm;
        }
        Ok(())
    }
}
