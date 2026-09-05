use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use super::terminal::{RawModeGuard, attempt_restore};

#[test]
fn restore_attempts_leave_screen_even_when_showing_the_cursor_fails() {
    let mut calls = Vec::new();
    let error = attempt_restore(
        &mut calls,
        |calls: &mut Vec<_>| {
            calls.push("show cursor");
            Err(io::Error::other("cursor failed"))
        },
        |calls: &mut Vec<_>| {
            calls.push("leave screen");
            Ok(())
        },
        || Ok(()),
    )
    .unwrap_err();
    assert_eq!(calls, ["show cursor", "leave screen"]);
    assert!(
        error.to_string().contains("show terminal cursor"),
        "{error:#}"
    );
}

#[test]
fn restore_attempts_raw_mode_even_when_screen_cleanup_fails() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let target_calls = Rc::clone(&calls);
    let screen_calls = Rc::clone(&calls);
    let raw_calls = Rc::clone(&calls);
    let error = attempt_restore(
        &mut (),
        move |_| {
            target_calls.borrow_mut().push("show cursor");
            Err(io::Error::other("cursor failed"))
        },
        move |_| {
            screen_calls.borrow_mut().push("leave screen");
            Err(io::Error::other("screen failed"))
        },
        move || {
            raw_calls.borrow_mut().push("disable raw");
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(
        *calls.borrow(),
        ["show cursor", "leave screen", "disable raw"]
    );
    assert!(
        error.to_string().contains("show terminal cursor"),
        "{error:#}"
    );
}

#[test]
fn raw_mode_guard_restores_once_on_normal_finish_and_on_drop() {
    assert_eq!(restoration_count(true), 1);
    assert_eq!(restoration_count(false), 1);
}

#[test]
fn raw_mode_guard_retries_on_drop_when_explicit_restoration_fails() {
    let calls = Rc::new(RefCell::new(0));
    {
        let calls_for_guard = Rc::clone(&calls);
        let mut guard = RawModeGuard::new(true, move || {
            *calls_for_guard.borrow_mut() += 1;
            if *calls_for_guard.borrow() == 1 {
                Err(io::Error::other("temporary restore failure"))
            } else {
                Ok(())
            }
        });
        assert!(guard.restore().is_err());
    }
    assert_eq!(*calls.borrow(), 2);
}

fn restoration_count(explicit: bool) -> usize {
    let calls = Rc::new(RefCell::new(0));
    {
        let guard_calls = Rc::clone(&calls);
        let mut guard = RawModeGuard::new(true, move || {
            *guard_calls.borrow_mut() += 1;
            Ok(())
        });
        if explicit {
            guard.restore().unwrap();
        }
    }
    *calls.borrow()
}
