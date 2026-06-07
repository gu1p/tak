#![cfg(test)]

use crate::engine::protocol_result_http::request::{RemoteHttpResponse, ResponseHeader};

#[test]
fn legacy_wormhole_route_with_protobuf_content_type_marks_support() {
    let response = response_with_headers(vec![("content-type", "application/x-protobuf")]);

    assert!(super::marks_wormhole_support(&response));
}

#[test]
fn stream_status_response_without_headers_does_not_mark_support() {
    let response = response_with_headers(Vec::new());

    assert!(!super::marks_wormhole_support(&response));
}

fn response_with_headers(headers: Vec<(&str, &str)>) -> RemoteHttpResponse {
    RemoteHttpResponse {
        status: 200,
        headers: headers
            .into_iter()
            .map(|(name, value)| ResponseHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect(),
        body: Vec::new(),
        daemon_task_handle: None,
        daemon_peer_node_id: None,
        daemon_peer_endpoint: None,
    }
}
