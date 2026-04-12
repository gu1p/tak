use prost::Message;
use tak_proto::PollTaskEventsResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

pub fn wait_for_terminal_events(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) {
    let path = format!("/v1/tasks/{task_run_id}/events");
    for _ in 0..50 {
        let events =
            handle_remote_v1_request(context, store, "GET", &path, None).expect("events response");
        let events = PollTaskEventsResponse::decode(events.body.as_slice()).expect("decode events");
        if events.done {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("timed out waiting for terminal remote events");
}
