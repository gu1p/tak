use prost::Message;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

use super::parse_remote_events_response;
use crate::remote_protocol_codec::submit_payload_test_support::direct_target;

#[test]
fn parser_surfaces_terminal_failure_messages_as_status_messages() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 7,
            kind: "TASK_FAILED".into(),
            timestamp_ms: 10,
            success: Some(false),
            exit_code: Some(1),
            message: Some("worker exited before returning a result".into()),
            chunk: None,
            chunk_bytes: Vec::new(),
        }],
        done: true,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse remote events");

    assert_eq!(
        parsed.status_messages,
        vec!["worker exited before returning a result"]
    );
}

#[test]
fn parser_synthesizes_terminal_failure_status_from_exit_code() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 7,
            kind: "TASK_FAILED".into(),
            timestamp_ms: 10,
            success: Some(false),
            exit_code: Some(137),
            message: None,
            chunk: None,
            chunk_bytes: Vec::new(),
        }],
        done: true,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse remote events");

    assert_eq!(
        parsed.status_messages,
        vec!["remote task failed with exit code 137"]
    );
}

#[test]
fn parser_synthesizes_terminal_cancelled_status_from_exit_code() {
    let body = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 7,
            kind: "TASK_CANCELLED".into(),
            timestamp_ms: 10,
            success: Some(false),
            exit_code: Some(137),
            message: None,
            chunk: None,
            chunk_bytes: Vec::new(),
        }],
        done: true,
    }
    .encode_to_vec();

    let parsed =
        parse_remote_events_response(&direct_target(None), &body, 0).expect("parse remote events");

    assert_eq!(
        parsed.status_messages,
        vec!["remote task cancelled with exit code 137"]
    );
}
