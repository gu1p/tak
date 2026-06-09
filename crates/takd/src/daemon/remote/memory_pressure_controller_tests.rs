#![cfg(test)]

use super::*;

fn run(id: &str, created: i64, has_timeout: bool) -> ManagedContainer {
    ManagedContainer {
        id: id.to_string(),
        created,
        has_timeout,
        paused: false,
    }
}

fn paused(id: &str, created: i64) -> ManagedContainer {
    ManagedContainer {
        id: id.to_string(),
        created,
        has_timeout: false,
        paused: true,
    }
}

fn settings() -> MemoryPressureSettings {
    MemoryPressureSettings::defaults()
}

const GIB: u64 = 1024 * 1024 * 1024;

#[test]
fn thresholds_keep_emergency_below_pause_below_resume() {
    for total in [4 * GIB, 8 * GIB, 32 * GIB, 256 * GIB] {
        let th = thresholds(&settings(), total);
        assert!(th.emergency < th.pause, "total={total} {th:?}");
        assert!(th.pause < th.resume, "total={total} {th:?}");
        assert!(th.resume < total, "total={total} {th:?}");
    }
}

#[test]
fn thresholds_apply_percentage_on_large_nodes() {
    // 100 GiB: 15% = 15 GiB exceeds the 2 GiB floor and is below total/2.
    let total = 100 * GIB;
    let th = thresholds(&settings(), total);
    assert_eq!(th.pause, total / 100 * 15);
}

#[test]
fn thresholds_use_floor_when_percentage_is_tiny() {
    // 8 GiB: 15% = 1.2 GiB < 2 GiB floor -> the floor wins.
    let total = 8 * GIB;
    let floor = settings().pause_floor_mb * BYTES_PER_MB;
    let th = thresholds(&settings(), total);
    assert_eq!(th.pause, floor.min(total / 2));
}

#[test]
fn classify_covers_each_band_including_dead_band() {
    let th = Thresholds {
        emergency: 100,
        pause: 200,
        resume: 400,
    };
    assert_eq!(classify(50, &th), PressureState::Emergency);
    assert_eq!(classify(150, &th), PressureState::Pause);
    assert_eq!(classify(300, &th), PressureState::Normal);
    assert_eq!(classify(500, &th), PressureState::Resume);
}

#[test]
fn pauses_newest_running_container() {
    let running = vec![
        run("old", 10, false),
        run("mid", 20, false),
        run("new", 30, false),
    ];
    assert_eq!(
        select_pause_victims(&running, 1, 1),
        vec!["new".to_string()]
    );
}

#[test]
fn never_pauses_the_oldest_running_container() {
    let running = vec![run("old", 10, false), run("new", 20, false)];
    assert_eq!(
        select_pause_victims(&running, 1, 8),
        vec!["new".to_string()]
    );
    // Once only the oldest is left running, nothing else is pausable.
    assert!(select_pause_victims(&[run("old", 10, false)], 1, 8).is_empty());
}

#[test]
fn respects_min_running() {
    let running = vec![
        run("a", 10, false),
        run("b", 20, false),
        run("c", 30, false),
    ];
    // min_running=2 with 3 running -> at most one pause (newest).
    assert_eq!(select_pause_victims(&running, 2, 8), vec!["c".to_string()]);
}

#[test]
fn skips_timeout_bearing_container() {
    let running = vec![
        run("old", 10, false),
        run("timeout", 30, true),
        run("mid", 20, false),
    ];
    // Newest is timeout-bearing -> skipped; next-newest non-oldest is "mid".
    assert_eq!(
        select_pause_victims(&running, 1, 1),
        vec!["mid".to_string()]
    );
}

#[test]
fn emergency_pauses_newest_first_excluding_oldest_and_timeout() {
    let running = vec![
        run("oldest", 10, false),
        run("t", 15, true),
        run("a", 20, false),
        run("b", 30, false),
    ];
    assert_eq!(
        select_pause_victims(&running, 1, usize::MAX),
        vec!["b".to_string(), "a".to_string()]
    );
}

#[test]
fn unpause_targets_newest_paused_first() {
    let frozen = vec![paused("a", 10), paused("b", 30), paused("c", 20)];
    assert_eq!(select_unpause_target(&frozen), Some("b".to_string()));
    assert_eq!(select_unpause_target(&[]), None);
}

#[test]
fn managed_containers_parses_timeout_running_and_paused_state() {
    let running_summary = ContainerSummary {
        id: Some("c1".to_string()),
        created: Some(42),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::from([
            ("tak.owner".to_string(), "takd".to_string()),
            ("tak.timeout_s".to_string(), "30".to_string()),
        ])),
        ..Default::default()
    };
    let paused_summary = ContainerSummary {
        id: Some("c2".to_string()),
        created: Some(7),
        state: Some("paused".to_string()),
        labels: Some(std::collections::HashMap::new()),
        ..Default::default()
    };
    let parsed = managed_containers(&[running_summary, paused_summary]);
    assert_eq!(
        parsed,
        vec![
            ManagedContainer {
                id: "c1".to_string(),
                created: 42,
                has_timeout: true,
                paused: false,
            },
            ManagedContainer {
                id: "c2".to_string(),
                created: 7,
                has_timeout: false,
                paused: true,
            },
        ]
    );
}

#[test]
fn managed_containers_treats_zero_or_missing_timeout_as_pausable() {
    let zero = ContainerSummary {
        id: Some("z".to_string()),
        created: Some(1),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::from([(
            "tak.timeout_s".to_string(),
            "0".to_string(),
        )])),
        ..Default::default()
    };
    let missing = ContainerSummary {
        id: Some("m".to_string()),
        created: Some(2),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::new()),
        ..Default::default()
    };
    let parsed = managed_containers(&[zero, missing]);
    assert!(parsed.iter().all(|c| !c.has_timeout), "{parsed:?}");
}

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
