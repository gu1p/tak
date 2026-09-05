use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Style;

use super::model::DashboardState;
use super::navigation::DashboardNavigation;

pub(super) struct StyledFrame(Buffer);

impl StyledFrame {
    pub(super) fn style_for(&self, needle: &str) -> Style {
        let area = self.0.area;
        for y in area.y..area.y.saturating_add(area.height) {
            let row = (area.x..area.x.saturating_add(area.width))
                .map(|x| self.0[(x, y)].symbol())
                .collect::<String>();
            if let Some(byte_column) = row.find(needle) {
                let column = row[..byte_column].chars().count();
                let x = area.x + u16::try_from(column).expect("dashboard column fits");
                return self.0[(x, y)].style();
            }
        }
        panic!("missing {needle:?} in dashboard");
    }
}

pub(super) fn styled_frame(state: &DashboardState, width: u16, color_enabled: bool) -> StyledFrame {
    let height = state
        .jobs
        .len()
        .saturating_add(state.nodes.len())
        .saturating_add(state.logs.len().min(8))
        .saturating_add(18)
        .clamp(30, 60);
    styled_frame_at_size(
        state,
        width,
        u16::try_from(height).expect("test height fits"),
        color_enabled,
    )
}

pub(super) fn styled_frame_at_size(
    state: &DashboardState,
    width: u16,
    height: u16,
    color_enabled: bool,
) -> StyledFrame {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test dashboard terminal");
    let navigation = DashboardNavigation::default();
    terminal
        .draw(|frame| super::render::draw_with_navigation(frame, state, &navigation, color_enabled))
        .expect("draw test dashboard");
    StyledFrame(terminal.backend().buffer().clone())
}

pub(super) fn frame(state: &DashboardState, width: u16) -> String {
    text(styled_frame(state, width, false))
}

pub(super) fn frame_at_size(state: &DashboardState, width: u16, height: u16) -> String {
    text(styled_frame_at_size(state, width, height, false))
}

pub(super) fn frame_at_size_with_navigation(
    state: &DashboardState,
    navigation: &DashboardNavigation,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test dashboard terminal");
    terminal
        .draw(|frame| super::render::draw_with_navigation(frame, state, navigation, false))
        .expect("draw navigated test dashboard");
    text(StyledFrame(terminal.backend().buffer().clone()))
}

fn text(rendered: StyledFrame) -> String {
    let area = rendered.0.area;
    let mut lines = Vec::with_capacity(area.height.into());
    for y in area.y..area.y.saturating_add(area.height) {
        let line = (area.x..area.x.saturating_add(area.width))
            .map(|x| rendered.0[(x, y)].symbol())
            .collect::<String>();
        lines.push(line.trim_end().to_owned());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}
