use super::*;

pub(crate) fn wait_for_task_event(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    kind: &str,
) -> Vec<tak_proto::RemoteEvent> {
    let deadline = Instant::now() + REMOTE_ADMISSION_WAIT_TIMEOUT;
    loop {
        let events = task_events(context, store, task_run_id);
        if events.iter().any(|event| event.kind == kind) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {kind} event for {task_run_id}: {events:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
