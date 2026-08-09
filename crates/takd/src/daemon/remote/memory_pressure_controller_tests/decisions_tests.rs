use super::super::policy::{TickAction, decide};
use super::super::pressure::PressureState;
use super::{paused, run};

#[test]
fn forced_progress_unpauses_when_below_min_running_even_in_emergency() {
    // Nothing running, work paused, memory critical: must still unpause to drain,
    // otherwise the node freezes forever (paused RSS never frees).
    let frozen = vec![paused("a", 10), paused("b", 30)];
    assert_eq!(
        decide(PressureState::Emergency, &[], &frozen, 1),
        TickAction::Unpause("b".to_string())
    );
}

#[test]
fn forced_progress_takes_priority_over_pausing() {
    let frozen = vec![paused("p", 5)];
    assert_eq!(
        decide(PressureState::Pause, &[], &frozen, 1),
        TickAction::Unpause("p".to_string())
    );
}

#[test]
fn decide_emergency_pauses_newest_first() {
    let running = vec![
        run("oldest", 10, false),
        run("a", 20, false),
        run("b", 30, false),
    ];
    assert_eq!(
        decide(PressureState::Emergency, &running, &[], 1),
        TickAction::Pause(vec!["b".to_string(), "a".to_string()])
    );
}

#[test]
fn decide_pause_band_pauses_single_newest() {
    let running = vec![run("old", 10, false), run("new", 20, false)];
    assert_eq!(
        decide(PressureState::Pause, &running, &[], 1),
        TickAction::Pause(vec!["new".to_string()])
    );
}

#[test]
fn decide_resume_unpauses_newest_paused() {
    let running = vec![run("x", 5, false)];
    let frozen = vec![paused("a", 10), paused("b", 30)];
    assert_eq!(
        decide(PressureState::Resume, &running, &frozen, 1),
        TickAction::Unpause("b".to_string())
    );
}

#[test]
fn decide_normal_is_no_action() {
    let running = vec![run("a", 10, false), run("b", 20, false)];
    assert_eq!(
        decide(PressureState::Normal, &running, &[], 1),
        TickAction::None
    );
}

#[test]
fn decide_no_action_when_only_protected_runner_and_nothing_paused() {
    let running = vec![run("only", 10, false)];
    assert_eq!(
        decide(PressureState::Emergency, &running, &[], 1),
        TickAction::None
    );
}
