use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use super::navigation::{DashboardNavigation, NavigationAction, Panel, key_to_action};

#[test]
fn navigation_keys_are_accessible_and_ignore_non_press_events() {
    for (code, modifiers, expected) in [
        (
            KeyCode::Tab,
            KeyModifiers::NONE,
            NavigationAction::NextPanel,
        ),
        (
            KeyCode::BackTab,
            KeyModifiers::SHIFT,
            NavigationAction::PreviousPanel,
        ),
        (KeyCode::Up, KeyModifiers::NONE, NavigationAction::LineUp),
        (
            KeyCode::Char('k'),
            KeyModifiers::NONE,
            NavigationAction::LineUp,
        ),
        (
            KeyCode::Down,
            KeyModifiers::NONE,
            NavigationAction::LineDown,
        ),
        (
            KeyCode::Char('j'),
            KeyModifiers::NONE,
            NavigationAction::LineDown,
        ),
        (
            KeyCode::PageUp,
            KeyModifiers::NONE,
            NavigationAction::PageUp,
        ),
        (
            KeyCode::PageDown,
            KeyModifiers::NONE,
            NavigationAction::PageDown,
        ),
        (KeyCode::Home, KeyModifiers::NONE, NavigationAction::Home),
        (KeyCode::End, KeyModifiers::NONE, NavigationAction::End),
        (
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            NavigationAction::Interrupt,
        ),
    ] {
        let key = KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Press);
        assert_eq!(key_to_action(key), Some(expected));
    }

    let released = KeyEvent {
        code: KeyCode::End,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: KeyEventState::NONE,
    };
    assert_eq!(key_to_action(released), None);
}

#[test]
fn panel_focus_wraps_and_each_panel_retains_its_scroll_position() {
    let mut navigation = DashboardNavigation::default();
    assert_eq!(navigation.focus(), Panel::Tasks);
    navigation.apply(NavigationAction::PageDown);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 8);

    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::LineDown);
    assert_eq!(navigation.focus(), Panel::Queue);
    assert_eq!(navigation.scroll_offset(Panel::Queue, 100, 10), 1);

    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::NextPanel);
    assert_eq!(navigation.focus(), Panel::Tasks);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 8);

    navigation.apply(NavigationAction::PreviousPanel);
    assert_eq!(navigation.focus(), Panel::Nodes);
}

#[test]
fn home_end_and_up_down_saturate_without_losing_tail_following() {
    let mut navigation = DashboardNavigation::default();
    navigation.apply(NavigationAction::LineUp);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 0);
    navigation.apply(NavigationAction::End);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 90);
    navigation.apply(NavigationAction::LineUp);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 89);
    navigation.apply(NavigationAction::LineDown);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 90);
    navigation.apply(NavigationAction::Home);
    assert_eq!(navigation.scroll_offset(Panel::Tasks, 100, 10), 0);
}
