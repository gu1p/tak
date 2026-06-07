use crate::engine::protocol_result_http::request::RemoteHttpResponse;

pub(super) fn marks_wormhole_support(response: &RemoteHttpResponse) -> bool {
    has_wormhole_marker(response) || has_protobuf_content_type(response)
}

fn has_wormhole_marker(response: &RemoteHttpResponse) -> bool {
    response.header("x-tak-workspace-transfer") == Some("wormhole")
}

fn has_protobuf_content_type(response: &RemoteHttpResponse) -> bool {
    response
        .header("content-type")
        .is_some_and(is_protobuf_content_type)
}

fn is_protobuf_content_type(value: &str) -> bool {
    value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("application/x-protobuf")
}

#[path = "support_tests.rs"]
mod tests;
