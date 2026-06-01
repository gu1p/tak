use prost::Message;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

use super::parse_remote_events_response;
use crate::TaskStatusEventKind;
use crate::remote_protocol_codec::submit_payload_test_support::direct_target;

#[test]
fn parser_surfaces_queue_events_as_structured_status_updates() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 7,
            kind: "TASK_QUEUED".into(),
            timestamp_ms: 10,
            success: None,
            exit_code: None,
            message: Some(
                "queued: waiting for remote capacity (queue position: 3; 2 tasks ahead)".into(),
            ),
            chunk: None,
            chunk_bytes: Vec::new(),
        }],
        done: false,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse remote events");

    assert_eq!(
        parsed.status_messages,
        vec!["queued: waiting for remote capacity (queue position: 3; 2 tasks ahead)"]
    );
    assert_eq!(parsed.status_updates.len(), 1);
    assert_eq!(
        parsed.status_updates[0].kind,
        TaskStatusEventKind::QueueAdmission
    );
    assert_eq!(parsed.status_updates[0].queue_position, Some(3));
}
