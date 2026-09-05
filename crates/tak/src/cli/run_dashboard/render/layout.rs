use ratatui::text::Line;

use super::super::model::DashboardState;
use super::super::navigation::Panel;
use super::{enabled, lifecycle_style, text};

pub(super) fn panel_heights(lengths: [usize; 4], available: u16, focus: Panel) -> [u16; 4] {
    let mut heights = [3, 5, 2, 4];
    let focus_index = match focus {
        Panel::Nodes => 0,
        Panel::Tasks => 1,
        Panel::Queue => 2,
        Panel::Logs => 3,
    };
    if available < heights.into_iter().sum() {
        heights = [0; 4];
        heights[focus_index] = available;
        return heights;
    }
    let caps = [
        if focus == Panel::Nodes {
            available / 2
        } else {
            available / 5
        }
        .max(3),
        (available / 3).max(5),
        if focus == Panel::Queue {
            available / 3
        } else {
            available / 5
        }
        .max(2),
    ];
    for index in 0..3 {
        let desired = u16::try_from(lengths[index].saturating_add(1)).unwrap_or(u16::MAX);
        let free = available.saturating_sub(heights.into_iter().sum());
        heights[index] = desired.min(caps[index]).min(heights[index] + free);
    }
    heights[3] += available.saturating_sub(heights.into_iter().sum());
    heights
}

pub(super) fn empty_state(state: &DashboardState, color: bool, width: u16) -> Vec<Line<'static>> {
    let detail = if let Some(error) = &state.error {
        error.as_str()
    } else if state.lifecycle == "loading" {
        "Waiting for persisted run state…"
    } else {
        "No executable task steps."
    };
    text::lines(
        detail,
        width,
        enabled(lifecycle_style(&state.lifecycle), color),
    )
}
