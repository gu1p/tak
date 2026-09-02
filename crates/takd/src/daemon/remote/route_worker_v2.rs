use super::*;

mod attempts;
mod identity;
mod observations;
mod snapshot;
#[cfg(test)]
mod snapshot_advisory_cache_tests;
mod workspace_cache;

const VERSION_HEADER: &str = "x-tak-protocol-version";

pub(super) fn handle_worker_v2_route(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> Option<Result<WorkerHttpResponse>> {
    if path != "/v2/worker/identity"
        && path != "/v2/worker/snapshot"
        && !observations::matches(path)
        && !path.starts_with("/v2/attempts/")
        && !path.starts_with("/v2/workspaces/cache/")
    {
        return None;
    }
    if !has_exact_v2_header(headers) {
        return Some(Ok(text_response(426, PROTOCOL_V2_UPGRADE_MESSAGE)));
    }
    if path == "/v2/worker/snapshot" {
        return Some(Ok(snapshot::handle(context, method)));
    }
    if path == "/v2/worker/identity" {
        return Some(Ok(identity::handle(context, method)));
    }
    if observations::matches(path) {
        return Some(observations::handle(context, store, method, path, query));
    }
    if method != "POST" {
        return Some(Ok(text_response(405, "method_not_allowed")));
    }
    if !has_json_content_type(headers) {
        return Some(Ok(text_response(400, "application_json_required")));
    }
    let body = body.unwrap_or_default();
    if path.starts_with("/v2/workspaces/cache/") {
        return Some(workspace_cache::handle(context, method, path, body));
    }
    Some(attempts::handle(context, store, path, body))
}

fn has_exact_v2_header(headers: &[(String, String)]) -> bool {
    let mut values = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(VERSION_HEADER))
        .map(|(_, value)| value.trim());
    values
        .next()
        .is_some_and(|value| value.eq_ignore_ascii_case("v2"))
        && values.next().is_none()
}

fn has_json_content_type(headers: &[(String, String)]) -> bool {
    let mut values = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.split(';').next().unwrap_or_default().trim());
    values
        .next()
        .is_some_and(|value| value.eq_ignore_ascii_case("application/json"))
        && values.next().is_none()
}
