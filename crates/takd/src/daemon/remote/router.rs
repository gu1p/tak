use super::*;

/// Handles one authenticated worker HTTP request, including transport headers.
///
/// ```rust
/// # use takd::{RemoteNodeContext, SubmitAttemptStore};
/// # use takd::daemon::remote::handle_worker_http_request;
/// # fn example(
/// #     context: &RemoteNodeContext,
/// #     store: &SubmitAttemptStore,
/// # ) -> anyhow::Result<()> {
/// let response = handle_worker_http_request(
///     context,
///     store,
///     "GET",
///     "/v1/node/info",
///     &[],
///     None,
/// )?;
/// # let _ = response;
/// # Ok(())
/// # }
/// ```
pub fn handle_worker_http_request(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> Result<WorkerHttpResponse> {
    let method = method.trim().to_ascii_uppercase();
    let (path_only, query) = split_path_and_query(path);

    if let Some(response) = reject_v1_route(path_only) {
        return Ok(response);
    }

    if let Some(response) =
        handle_worker_v2_route(context, store, &method, path_only, query, headers, body)
    {
        return response;
    }
    Ok(error_response(
        404,
        &format!("not_found:{method}:{path_only}"),
    ))
}

fn reject_v1_route(path: &str) -> Option<WorkerHttpResponse> {
    path.starts_with("/v1/")
        .then(|| text_response(426, PROTOCOL_V2_UPGRADE_MESSAGE))
}
