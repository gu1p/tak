use std::cell::{Cell, RefCell};
use std::io;

use super::terminal::{CleanupOps, TerminalCleanup};

thread_local! {
    static CALLS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static FAIL_DISABLE: Cell<bool> = const { Cell::new(false) };
    static FAIL_RESTORE: Cell<bool> = const { Cell::new(false) };
}

#[test]
fn cleanup_runs_when_guard_is_dropped() {
    reset();
    {
        let _cleanup = TerminalCleanup::with_ops(CleanupOps {
            disable_raw_mode: disable_raw_mode_for_test,
            restore_screen: restore_screen_for_test,
        });
    }
    assert_eq!(calls(), vec!["disable_raw_mode", "restore_screen"]);
}

#[test]
fn finish_attempts_both_cleanup_steps_when_disable_raw_mode_fails() {
    reset();
    FAIL_DISABLE.with(|flag| flag.set(true));
    let err = TerminalCleanup::with_ops(CleanupOps {
        disable_raw_mode: disable_raw_mode_for_test,
        restore_screen: restore_screen_for_test,
    })
    .finish()
    .expect_err("cleanup should surface raw-mode restore failures");
    assert!(
        err.to_string().contains("disable raw mode"),
        "unexpected error: {err:#}"
    );
    assert_eq!(calls(), vec!["disable_raw_mode", "restore_screen"]);
}

#[test]
fn finish_surfaces_restore_screen_errors_without_running_cleanup_twice() {
    reset();
    FAIL_RESTORE.with(|flag| flag.set(true));
    let err = TerminalCleanup::with_ops(CleanupOps {
        disable_raw_mode: disable_raw_mode_for_test,
        restore_screen: restore_screen_for_test,
    })
    .finish()
    .expect_err("cleanup should surface screen restore failures");
    assert!(
        err.to_string().contains("restore terminal screen"),
        "unexpected error: {err:#}"
    );
    assert_eq!(calls(), vec!["disable_raw_mode", "restore_screen"]);
}

fn reset() {
    CALLS.with(|calls| calls.borrow_mut().clear());
    FAIL_DISABLE.with(|flag| flag.set(false));
    FAIL_RESTORE.with(|flag| flag.set(false));
}

fn calls() -> Vec<&'static str> {
    CALLS.with(|calls| calls.borrow().clone())
}

fn disable_raw_mode_for_test() -> io::Result<()> {
    CALLS.with(|calls| calls.borrow_mut().push("disable_raw_mode"));
    if FAIL_DISABLE.with(Cell::get) {
        Err(io::Error::other("disable failed"))
    } else {
        Ok(())
    }
}

fn restore_screen_for_test() -> io::Result<()> {
    CALLS.with(|calls| calls.borrow_mut().push("restore_screen"));
    if FAIL_RESTORE.with(Cell::get) {
        Err(io::Error::other("restore failed"))
    } else {
        Ok(())
    }
}
