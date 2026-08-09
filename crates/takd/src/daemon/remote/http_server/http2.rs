use super::*;

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioExecutor, TokioIo};
use prefixed_io::PrefixedIo;
use request::authorization_is_valid;

mod handler_error;
mod request_body;

use handler_error::sanitize_handler_detail;
use request_body::{collect_body_capped, declared_length_exceeds_cap};

// Server-side HTTP/2 receive windows. A job submit POSTs a multi-MB workspace as
// the request body; with the default 64 KiB window the client can only have one
// window in flight per onion round trip, throttling a 2.87 MB upload to ~45 RTTs
// (tens of seconds, and flaky against the phase-timeout floor on slow circuits).
// Advertising a window that comfortably exceeds a typical submit lets the whole
// body stream without per-window stalls, so the upload is bandwidth- not
// RTT-bound. Tor's own circuit/stream SENDME windows still apply underneath.
const HTTP2_STREAM_WINDOW: u32 = 4 * 1024 * 1024;
const HTTP2_CONNECTION_WINDOW: u32 = 8 * 1024 * 1024;

pub(super) async fn handle_remote_v1_http2_stream<S>(
    stream: PrefixedIo<S>,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let service = service_fn(move |request| {
        let store = store.clone();
        let context = context.clone();
        async move { handle_remote_v1_http2_request(request, store, context).await }
    });
    hyper::server::conn::http2::Builder::new(TokioExecutor::new())
        .initial_stream_window_size(HTTP2_STREAM_WINDOW)
        .initial_connection_window_size(HTTP2_CONNECTION_WINDOW)
        .serve_connection(TokioIo::new(stream), service)
        .await
        .context("serve http2 connection")
}

async fn handle_remote_v1_http2_request(
    request: HyperRequest<hyper::body::Incoming>,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> std::result::Result<HyperResponse<Full<Bytes>>, std::convert::Infallible> {
    let (parts, body) = request.into_parts();
    let authorization = parts
        .headers
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    // Authorize from the headers BEFORE buffering the body, so an unauthenticated
    // peer cannot make us allocate a large request body (the receive windows let
    // it arrive quickly) only to be rejected afterwards.
    if !authorization_is_valid(authorization, &context) {
        return Ok(hyper_response(error_response(401, "auth_failed")));
    }
    if declared_length_exceeds_cap(&parts.headers) {
        return Ok(hyper_response(error_response(
            413,
            "request_body_too_large",
        )));
    }
    let path = parts
        .uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let (path_only, query) = split_path_and_query(&path);
    if parts.method.as_str() == "POST"
        && path_only.starts_with("/v2/workspaces/uploads/")
        && path_only.ends_with("/stream")
    {
        let response =
            match stream_workspace_upload(&context, path_only, query, &parts.headers, body).await {
                Ok(response) => response,
                Err(err) => {
                    tracing::error!(error = %err, "workspace upload stream failed");
                    error_response(500, "workspace_upload_stream_failed")
                }
            };
        return Ok(hyper_response(response));
    }
    let body = match collect_body_capped(body).await {
        Ok(body) => body,
        Err(response) => return Ok(hyper_response(response)),
    };
    if parts.method.as_str() == "POST"
        && path_only.starts_with("/v2/workspaces/uploads/")
        && path_only.ends_with("/wormhole")
    {
        let response =
            match receive_workspace_wormhole_upload(&context, path_only, body.as_slice()).await {
                Ok(response) => response,
                Err(err) => {
                    tracing::error!(error = %err, "workspace wormhole upload failed");
                    error_response(500, "workspace_wormhole_upload_failed")
                }
            };
        return Ok(hyper_response(response));
    }
    let response = match handle_remote_v1_request(
        &context,
        &store,
        parts.method.as_str(),
        &path,
        &http2_headers(&parts.headers),
        (!body.is_empty()).then_some(body.as_slice()),
    ) {
        Ok(response) => response,
        Err(err) => {
            // Previously this collapsed every handler error to a bare, logless
            // `error_response(500, "request_failed")`, which made the real cause
            // (e.g. a transient SQLite busy/locked read) invisible on both the
            // daemon and the requesting client. Log the full chain locally and
            // hand the peer a sanitized, truncated form so a fatal 500 is
            // diagnosable over the authenticated channel.
            tracing::error!(
                error = %format!("{err:#}"),
                method = parts.method.as_str(),
                path = %path_only,
                "remote v1 request handler failed"
            );
            error_response(
                500,
                &format!("request_failed: {}", sanitize_handler_detail(&err)),
            )
        }
    };
    Ok(hyper_response(response))
}

fn http2_headers(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod http2_tests;

fn hyper_response(response: RemoteV1Response) -> HyperResponse<Full<Bytes>> {
    let mut builder = HyperResponse::builder()
        .status(response.status_code)
        .header(hyper::header::CONTENT_TYPE, response.content_type);
    for (name, value) in response.headers {
        builder = builder.header(name, value);
    }
    builder
        .body(Full::new(Bytes::from(response.body)))
        .unwrap_or_else(|_| {
            HyperResponse::builder()
                .status(500)
                .body(Full::new(Bytes::from_static(b"invalid_response")))
                .expect("fallback response")
        })
}
