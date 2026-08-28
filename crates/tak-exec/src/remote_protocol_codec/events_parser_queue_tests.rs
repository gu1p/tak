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
            queue_position: None,
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

#[test]
fn protobuf_queue_position_overrides_legacy_message_position() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 8,
            kind: "TASK_QUEUE_POSITION".into(),
            message: Some("queue position: 9; 8 tasks ahead".into()),
            queue_position: Some(2),
            ..RemoteEvent::default()
        }],
        done: false,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse events");

    assert_eq!(parsed.status_updates[0].queue_position, Some(2));
    assert_eq!(
        parsed.status_updates[0].kind,
        TaskStatusEventKind::QueuePositionChanged
    );
}

#[test]
fn worker_start_is_exposed_as_a_running_transition() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 9,
            kind: "TASK_STARTED".into(),
            ..RemoteEvent::default()
        }],
        done: false,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse events");

    assert_eq!(parsed.status_updates.len(), 1);
    assert_eq!(
        parsed.status_updates[0].kind,
        TaskStatusEventKind::RemoteExecutionStart
    );
}
