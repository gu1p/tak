use std::net::TcpStream;

use prost::Message;
use tak_proto::{ErrorResponse, SubmitTaskRequest};

use super::RecordingEvents;
use super::remote_responses::{error_response, submit_response};
use crate::support::http::{TestHttpRequest, write_protobuf_response};

#[derive(Clone, Copy)]
pub(super) enum SubmitBehavior {
    Success,
    Failure,
}

pub(super) fn handle_submit(
    stream: &mut TcpStream,
    events: &RecordingEvents,
    submit: SubmitBehavior,
    reap_after_reference: bool,
    request: &TestHttpRequest,
) {
    let payload = SubmitTaskRequest::decode(request.body.as_slice()).ok();
    let referenced_upload_id = payload
        .as_ref()
        .and_then(|p| p.workspace_upload.as_ref())
        .map(|upload| upload.upload_id.clone());
    // Mirror takd: a submit that references a workspace upload no longer present on the node
    // (reaped) is rejected with 409 so the client re-uploads.
    if let Some(upload_id) = referenced_upload_id.as_ref()
        && !events.is_upload_available(upload_id)
    {
        events.record_upload_conflict(upload_id);
        write_protobuf_response(
            stream,
            "409 Conflict",
            &error_response("workspace_upload_missing"),
        );
        return;
    }
    events.record("remote_submit");
    if let Some(payload) = payload {
        events.record_submit_payload(payload);
    }
    write_submit_response(stream, submit);
    // Simulate the cleanup janitor reaping the blob right after this reference, so a later
    // task reusing it must re-upload.
    if reap_after_reference && let Some(upload_id) = referenced_upload_id {
        events.reap_upload(&upload_id);
    }
}

fn write_submit_response(stream: &mut TcpStream, submit: SubmitBehavior) {
    match submit {
        SubmitBehavior::Success => {
            write_protobuf_response(stream, "200 OK", &submit_response("task-run-1:1"));
        }
        SubmitBehavior::Failure => {
            write_protobuf_response(
                stream,
                "500 Internal Server Error",
                &ErrorResponse {
                    message: "submit failed".into(),
                },
            );
        }
    }
}
