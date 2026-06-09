use std::net::TcpStream;

use prost::Message;
use tak_proto::BeginWorkspaceUploadRequest;

use super::RecordingEvents;
use super::remote_responses::{
    append_upload_response, begin_upload_response, error_response, finish_upload_response,
};
use super::upload_config::UploadMode;
use crate::support::http::{TestHttpRequest, write_protobuf_response};

pub(super) const UPLOAD_PREFIX: &str = "/v2/workspaces/uploads/";

pub(super) fn handle_workspace_upload(
    stream: &mut TcpStream,
    events: &RecordingEvents,
    upload_mode: UploadMode,
    request: &TestHttpRequest,
) {
    if upload_mode == UploadMode::LegacyInline404 {
        // Legacy node: no upload endpoint, so the client falls back to an inline submit.
        write_protobuf_response(stream, "404 Not Found", &error_response("not found"));
        return;
    }

    let suffix = &request.path[UPLOAD_PREFIX.len()..];
    if request.method == "POST" && suffix == "begin" {
        let upload_id = match BeginWorkspaceUploadRequest::decode(request.body.as_slice()) {
            Ok(begin) => format!("{}-{}-{}", begin.task_run_id, begin.attempt, begin.sha256),
            Err(_) => {
                write_protobuf_response(
                    stream,
                    "400 Bad Request",
                    &error_response("invalid begin"),
                );
                return;
            }
        };
        events.record_upload_begin(&upload_id);
        write_protobuf_response(stream, "200 OK", &begin_upload_response(&upload_id));
        return;
    }
    if request.method == "POST"
        && let Some(upload_id) = suffix.strip_suffix("/finish")
    {
        events.mark_upload_available(upload_id);
        write_protobuf_response(stream, "200 OK", &finish_upload_response(upload_id));
        return;
    }
    if request.method == "PATCH" {
        // /v2/workspaces/uploads/{id}?offset={n} — acknowledge the chunk by echoing the next
        // offset. The client advances by the bytes it sent and ignores `complete` here.
        let offset = upload_offset_from_query(&request.path).unwrap_or(0);
        let next = offset + request.body.len() as u64;
        write_protobuf_response(stream, "200 OK", &append_upload_response(next));
        return;
    }
    write_protobuf_response(stream, "404 Not Found", &error_response("not found"));
}

fn upload_offset_from_query(path: &str) -> Option<u64> {
    let query = path.split_once('?')?.1;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("offset="))?
        .parse()
        .ok()
}
