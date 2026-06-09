use std::net::TcpStream;
use std::time::Duration;

use tak_proto::{NodeStatusResponse, PollTaskEventsResponse};

use super::RecordingEvents;
use super::remote_responses::{error_response, node_info, submit_response, success_result};
use super::submit_route::{SubmitBehavior, handle_submit};
use super::upload_config::UploadConfig;
use super::upload_routes::{UPLOAD_PREFIX, handle_workspace_upload};
use crate::support::http::{read_request_path_and_body, write_protobuf_response};

/// How a recording node responds, captured once per server and reused for every request.
pub(super) struct RecordingResponses {
    pub(super) submit: SubmitBehavior,
    pub(super) upload: UploadConfig,
    pub(super) status: Option<NodeStatusResponse>,
    pub(super) result_delay: Duration,
}

pub(super) fn serve_remote_request(
    stream: &mut TcpStream,
    node_id: &str,
    port: u16,
    events: &RecordingEvents,
    responses: &RecordingResponses,
) -> bool {
    let Some(request) = read_request_path_and_body(stream) else {
        return true;
    };
    match request.path.as_str() {
        "/__shutdown" => {
            write_protobuf_response(stream, "200 OK", &submit_response("shutdown"));
            return false;
        }
        "/v1/node/info" => {
            write_protobuf_response(stream, "200 OK", &node_info(node_id, port));
            return true;
        }
        "/v1/node/status" => {
            match responses.status.as_ref() {
                Some(status) => write_protobuf_response(stream, "200 OK", status),
                None => {
                    write_protobuf_response(stream, "404 Not Found", &error_response("not found"))
                }
            }
            return true;
        }
        "/v1/tasks/submit" => {
            let reap = responses.upload.reap_after_reference;
            handle_submit(stream, events, responses.submit, reap, &request);
            return true;
        }
        _ => {}
    }

    if request.path.starts_with(UPLOAD_PREFIX) {
        handle_workspace_upload(stream, events, responses.upload.mode, &request);
        return true;
    }
    if request.path.contains("/events") {
        let response = PollTaskEventsResponse {
            events: Vec::new(),
            done: true,
        };
        write_protobuf_response(stream, "200 OK", &response);
        return true;
    }
    if request.path.contains("/result") {
        std::thread::sleep(responses.result_delay);
        write_protobuf_response(stream, "200 OK", &success_result(node_id));
        return true;
    }
    write_protobuf_response(stream, "404 Not Found", &error_response("not found"));
    true
}
