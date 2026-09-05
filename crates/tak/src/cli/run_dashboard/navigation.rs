use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const PAGE_LINES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Panel {
    Nodes,
    Tasks,
    Queue,
    Logs,
}

impl Panel {
    const ALL: [Self; 4] = [Self::Nodes, Self::Tasks, Self::Queue, Self::Logs];

    fn index(self) -> usize {
        match self {
            Self::Nodes => 0,
            Self::Tasks => 1,
            Self::Queue => 2,
            Self::Logs => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NavigationAction {
    NextPanel,
    PreviousPanel,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Home,
    End,
    Interrupt,
}

#[derive(Clone, Copy)]
enum Position {
    FromStart(usize),
    FromEnd(usize),
}

pub(super) struct DashboardNavigation {
    focus: Panel,
    positions: [Position; 4],
}

impl Default for DashboardNavigation {
    fn default() -> Self {
        Self {
            focus: Panel::Tasks,
            positions: [
                Position::FromStart(0),
                Position::FromStart(0),
                Position::FromStart(0),
                Position::FromEnd(0),
            ],
        }
    }
}

impl DashboardNavigation {
    pub(super) fn apply(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::NextPanel => self.move_focus(1),
            NavigationAction::PreviousPanel => self.move_focus(Panel::ALL.len() - 1),
            NavigationAction::LineUp => self.move_up(1),
            NavigationAction::LineDown => self.move_down(1),
            NavigationAction::PageUp => self.move_up(PAGE_LINES),
            NavigationAction::PageDown => self.move_down(PAGE_LINES),
            NavigationAction::Home => self.set_position(Position::FromStart(0)),
            NavigationAction::End => self.set_position(Position::FromEnd(0)),
            NavigationAction::Interrupt => {}
        }
    }

    pub(super) fn focus(&self) -> Panel {
        self.focus
    }

    pub(super) fn scroll_offset(&self, panel: Panel, total: usize, visible: usize) -> u16 {
        let maximum = total.saturating_sub(visible);
        let offset = match self.positions[panel.index()] {
            Position::FromStart(offset) => offset.min(maximum),
            Position::FromEnd(lines) => maximum.saturating_sub(lines),
        };
        u16::try_from(offset).unwrap_or(u16::MAX)
    }

    fn move_focus(&mut self, distance: usize) {
        self.focus = Panel::ALL[(self.focus.index() + distance) % Panel::ALL.len()];
    }

    fn move_up(&mut self, distance: usize) {
        self.set_position(match self.current_position() {
            Position::FromStart(offset) => Position::FromStart(offset.saturating_sub(distance)),
            Position::FromEnd(lines) => Position::FromEnd(lines.saturating_add(distance)),
        });
    }

    fn move_down(&mut self, distance: usize) {
        self.set_position(match self.current_position() {
            Position::FromStart(offset) => Position::FromStart(offset.saturating_add(distance)),
            Position::FromEnd(lines) => Position::FromEnd(lines.saturating_sub(distance)),
        });
    }

    fn current_position(&self) -> Position {
        self.positions[self.focus.index()]
    }

    fn set_position(&mut self, position: Position) {
        self.positions[self.focus.index()] = position;
    }
}

pub(super) fn key_to_action(key: KeyEvent) -> Option<NavigationAction> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
    {
        return (key.kind == KeyEventKind::Press).then_some(NavigationAction::Interrupt);
    }
    match key.code {
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Some(NavigationAction::PreviousPanel)
        }
        KeyCode::Tab => Some(NavigationAction::NextPanel),
        KeyCode::BackTab => Some(NavigationAction::PreviousPanel),
        KeyCode::Up | KeyCode::Char('k') => Some(NavigationAction::LineUp),
        KeyCode::Down | KeyCode::Char('j') => Some(NavigationAction::LineDown),
        KeyCode::PageUp => Some(NavigationAction::PageUp),
        KeyCode::PageDown => Some(NavigationAction::PageDown),
        KeyCode::Home => Some(NavigationAction::Home),
        KeyCode::End => Some(NavigationAction::End),
        _ => None,
    }
}
