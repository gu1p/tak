use std::net::TcpStream;

use tak_proto::{
    ErrorResponse, GetTaskResultResponse, NodeInfo, PollTaskEventsResponse, SubmitTaskResponse,
};

use super::super::super::http::{read_request_path, write_protobuf_response};
use super::ScriptedEventsState;

pub(super) fn serve_request(stream: &mut TcpStream, state: &mut ScriptedEventsState) -> bool {
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
                write_protobuf_response(stream, "404 Not Found", &result_not_ready());
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

fn node_info(node_id: &str, port: u16) -> NodeInfo {
    NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
    }
}

fn result_not_ready() -> ErrorResponse {
    ErrorResponse {
        message: "result_not_ready".into(),
    }
}
