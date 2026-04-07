use super::*;

/// Handles one remote V1 HTTP request and returns a fully formed daemon response.
///
/// ```no_run
/// # // Reason: This behavior depends on runtime request payloads and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn handle_remote_v1_request(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<RemoteV1Response> {
    let method = method.trim().to_ascii_uppercase();
    let (path_only, query) = split_path_and_query(path);

    if let Some(response) = handle_node_metadata_route(context, &method, path_only) {
        return Ok(response);
    }

    if method == "POST" && path_only == "/v1/tasks/submit" {
        return handle_remote_submit_route(context, store, body);
    }

    if let Some(response) = handle_remote_events_route(store, &method, path_only, query)? {
        return Ok(response);
    }
    if let Some(response) = handle_remote_outputs_route(store, &method, path_only, query)? {
        return Ok(response);
    }
    if let Some(response) = handle_remote_result_route(store, &method, path_only, query)? {
        return Ok(response);
    }

    if let Some(response) = handle_remote_cancel_route(&method, path_only) {
        return Ok(response);
    }

    Ok(error_response(
        404,
        &format!("not_found:{method}:{path_only}"),
    ))
}
