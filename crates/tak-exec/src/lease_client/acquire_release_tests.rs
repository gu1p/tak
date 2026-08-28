use std::time::Duration;

use super::coordination_status::{CoordinationQueueTracker, wait_for_retry_or_cancellation};
use crate::engine::{RunCancellation, TaskStatusEventKind, is_run_cancelled_error};

#[test]
fn coordination_queue_deduplicates_polls_and_reports_position_changes() {
    let mut tracker = CoordinationQueueTracker::default();
    assert_eq!(
        tracker.pending(3),
        Some(TaskStatusEventKind::QueueAdmission)
    );
    assert_eq!(tracker.pending(3), None);
    assert_eq!(
        tracker.pending(2),
        Some(TaskStatusEventKind::QueuePositionChanged)
    );
    assert_eq!(tracker.granted(), Some(TaskStatusEventKind::Dispatch));
    assert_eq!(tracker.granted(), None);
}

#[tokio::test]
async fn coordination_retry_wait_stops_immediately_when_cancelled() {
    let cancellation = RunCancellation::new();
    cancellation.cancel();
    let error = wait_for_retry_or_cancellation(Duration::from_secs(60), &cancellation)
        .await
        .expect_err("cancelled wait");
    assert!(is_run_cancelled_error(&error), "{error:#}");
}
