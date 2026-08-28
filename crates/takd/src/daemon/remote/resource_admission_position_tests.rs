#![cfg(test)]

use std::sync::mpsc;
use std::time::Duration;

use super::test_support::request;
use super::{ResourceCapacity, SharedResourceAdmission};

#[test]
fn waiting_worker_observes_each_changed_fifo_position_once() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 4.0,
        memory_mb: 4096,
    });
    admission
        .admit_or_queue(request("running", 4.0, 4096))
        .expect("running");
    admission
        .admit_or_queue(request("first", 4.0, 4096))
        .expect("first");
    admission
        .admit_or_queue(request("second", 4.0, 4096))
        .expect("second");

    let waiting = admission.clone();
    let cancellation = tak_runner::RunCancellation::default();
    let (positions_tx, positions_rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        waiting.wait_until_admitted_with_positions("second", &cancellation, |position| {
            positions_tx.send(position).expect("position receiver");
        })
    });

    assert_eq!(
        positions_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("initial"),
        2
    );
    admission.release("running").expect("promote first");
    assert_eq!(
        positions_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("changed"),
        1
    );
    admission.release("first").expect("promote second");
    thread.join().expect("wait thread").expect("admitted");
    assert!(
        positions_rx.try_recv().is_err(),
        "duplicate position callback"
    );
}
