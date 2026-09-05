use std::cell::Cell;
use std::rc::Rc;

use anyhow::anyhow;

use crate::cli::run_dashboard::{
    attempt_or_disable, disable_after_error, input_or_disable, start_or_disable,
};

struct TrackedDashboard(Rc<Cell<bool>>);

impl Drop for TrackedDashboard {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[test]
fn dashboard_failures_degrade_without_aborting_daemon_owned_work() {
    let unavailable = start_or_disable::<TrackedDashboard>(
        Err(anyhow!("initial terminal draw failed")),
        "before workspace upload",
    );
    assert!(unavailable.is_none());

    let dropped = Rc::new(Cell::new(false));
    let mut active = Some(TrackedDashboard(Rc::clone(&dropped)));
    disable_after_error(
        &mut active,
        anyhow!("navigation redraw failed"),
        "during workspace upload",
    );

    assert!(active.is_none());
    assert!(dropped.get(), "failed dashboard must release the terminal");
}

#[test]
fn failed_dashboard_render_returns_the_same_event_to_text_output() {
    let dropped = Rc::new(Cell::new(false));
    let mut active = Some(TrackedDashboard(Rc::clone(&dropped)));

    let handled = attempt_or_disable(
        &mut active,
        |_| Err::<bool, _>(anyhow!("event draw failed")),
        "while rendering an event",
    );

    assert_eq!(handled, None, "text renderer must receive the event");
    assert!(dropped.get(), "failed dashboard must release the terminal");
}

#[test]
fn failed_dashboard_input_keeps_the_durable_operation_in_flight() {
    let dropped = Rc::new(Cell::new(false));
    let mut active = Some(TrackedDashboard(Rc::clone(&dropped)));

    let was_interrupt = input_or_disable(
        &mut active,
        Err(anyhow!("input redraw failed")),
        "while cancellation is pending",
    );

    assert!(!was_interrupt, "failure is not a second Ctrl-C");
    assert!(active.is_none());
    assert!(dropped.get());
}
