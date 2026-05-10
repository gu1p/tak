use std::net::TcpStream;

use tak_proto::{ErrorResponse, PollTaskEventsResponse};

use super::RecordingEvents;
use super::remote_responses::{node_info, submit_response, success_result};
use crate::support::http::{read_request_path, write_protobuf_response};

#[derive(Clone, Copy)]
pub(super) enum SubmitBehavior {
    Success,
    Failure,
}

pub(super) fn serve_remote_request(
    stream: &mut TcpStream,
    node_id: &str,
    port: u16,
    events: &RecordingEvents,
    submit: SubmitBehavior,
) -> bool {
    let Some(path) = read_request_path(stream) else {
        return true;
    };
    match path.as_str() {
        "/__shutdown" => {
            write_protobuf_response(stream, "200 OK", &submit_response("shutdown"));
            false
        }
        "/v1/node/info" => {
            write_protobuf_response(stream, "200 OK", &node_info(node_id, port));
            true
        }
        "/v1/tasks/submit" => {
            events.record("remote_submit");
            write_submit_response(stream, submit);
            true
        }
        _ if path.contains("/events") => {
            write_protobuf_response(
                stream,
                "200 OK",
                &PollTaskEventsResponse {
                    events: Vec::new(),
                    done: true,
                },
            );
            true
        }
        _ if path.contains("/result") => {
            write_protobuf_response(stream, "200 OK", &success_result(node_id));
            true
        }
        _ => {
            write_protobuf_response(
                stream,
                "404 Not Found",
                &ErrorResponse {
                    message: "not found".into(),
                },
            );
            true
        }
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
