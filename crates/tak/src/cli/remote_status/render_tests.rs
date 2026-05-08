use ratatui::style::{Color, Modifier};

use crate::cli::remote_status::view::RemoteStatusView;

use super::render_test_support::{
    error_result, ok_result, remote, render_dashboard_buffer, render_dashboard_text,
    style_for_text, warning_result,
};

#[test]
fn dashboard_renders_progress_rows_and_active_jobs() {
    let mut view =
        RemoteStatusView::checking(&[remote("builder-b"), remote("builder-a")], 1, false);
    view.mark_complete(ok_result("builder-b", true));

    let text = render_dashboard_text(&view, false);

    assert!(
        text.contains("Remote Status"),
        "missing dashboard title:\n{text}"
    );
    assert!(text.contains("CHECKING"), "missing loading state:\n{text}");
    assert!(
        text.contains("[===="),
        "missing per-node progress bar:\n{text}"
    );
    assert!(text.contains("builder-a"), "missing pending node:\n{text}");
    assert!(text.contains("BUSY"), "missing busy status:\n{text}");
    assert!(
        text.contains("Active Jobs"),
        "missing active jobs section:\n{text}"
    );
    assert!(
        text.contains("//apps/web:build"),
        "missing active job:\n{text}"
    );
}

#[test]
fn dashboard_colors_status_badges_semantically() {
    let mut view = RemoteStatusView::checking(
        &[
            remote("builder-ok"),
            remote("builder-warn"),
            remote("builder-error"),
        ],
        1,
        false,
    );
    view.mark_complete(ok_result("builder-ok", false));
    view.mark_complete(warning_result("builder-warn"));
    view.mark_complete(error_result("builder-error"));

    let buffer = render_dashboard_buffer(&view, true);

    assert_eq!(style_for_text(&buffer, "OK").fg, Some(Color::Green));
    assert!(
        style_for_text(&buffer, "OK")
            .add_modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(style_for_text(&buffer, "WARN").fg, Some(Color::Yellow));
    assert_eq!(style_for_text(&buffer, "ERROR").fg, Some(Color::Red));
}
