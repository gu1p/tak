use tak_proto::{ErrorResponse, PollTaskEventsResponse, SubmitTaskResponse};

use super::super::super::http::{read_request_path, write_protobuf_response};
use super::{
    scripted_events_state::ScriptedEventsState,
    scripted_node_status::{node_info, node_status, status_unavailable},
};

pub(super) fn serve_request(
    stream: &mut std::net::TcpStream,
    state: &mut ScriptedEventsState,
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
            write_protobuf_response(stream, "200 OK", &node_info(&state.node_id, state.port));
            true
        }
        "/v1/node/status" => {
            if state.status_available {
                write_protobuf_response(stream, "200 OK", &node_status(&state.node_id, state.port));
            } else {
                write_protobuf_response(stream, "500 Internal Server Error", &status_unavailable());
            }
            true
        }
        "/v1/tasks/submit" => {
            write_protobuf_response(stream, "200 OK", &submit_response("task-run-1:1"));
            true
        }
        _ if path.contains("/events") => {
            let plan = state
                .plans
                .get(state.event_calls)
                .cloned()
                .unwrap_or_else(|| state.fallback_plan.clone());
            state.event_calls += 1;
            if !plan.delay.is_zero() {
                std::thread::sleep(plan.delay);
            }
            write_protobuf_response(
                stream,
                "200 OK",
                &PollTaskEventsResponse {
                    events: plan.events,
                    done: plan.done,
                },
            );
            true
        }
        _ if path.contains("/result") => {
            if state.event_calls >= state.result_ready_after_event_calls {
                write_protobuf_response(stream, "200 OK", &state.result);
            } else {
                write_protobuf_response(
                    stream,
                    "404 Not Found",
                    &ErrorResponse {
                        message: "result_not_ready".into(),
                    },
                );
            }
            true
        }
        _ => {
            write_protobuf_response(stream, "404 Not Found", &submit_response("shutdown"));
            true
        }
    }
}

fn submit_response(idempotency_key: &str) -> SubmitTaskResponse {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: idempotency_key.into(),
        remote_worker: true,
    }
}
