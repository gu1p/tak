use prost::Message;
use tak_proto::PollTaskEventsResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

const REMOTE_EVENT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const REMOTE_EVENT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);

pub fn wait_for_terminal_events(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) {
    let path = format!("/v1/tasks/{task_run_id}/events");
    let deadline = std::time::Instant::now() + REMOTE_EVENT_WAIT_TIMEOUT;
    let last_events = loop {
        let events =
            handle_remote_v1_request(context, store, "GET", &path, None).expect("events response");
        let events = PollTaskEventsResponse::decode(events.body.as_slice()).expect("decode events");
        if events.done {
            return;
        }
        if std::time::Instant::now() >= deadline {
            break events.events;
        }
        std::thread::sleep(REMOTE_EVENT_POLL_INTERVAL);
    };
    panic!("timed out waiting for terminal remote events: {last_events:?}");
}
