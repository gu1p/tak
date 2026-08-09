use super::super::policy::{select_pause_victims, select_unpause_target};
use super::{paused, run};

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
