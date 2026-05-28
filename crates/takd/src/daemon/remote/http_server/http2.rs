use super::*;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioExecutor, TokioIo};
use prefixed_io::PrefixedIo;
use request::request_is_authorized;

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
        .serve_connection(TokioIo::new(stream), service)
        .await
        .context("serve http2 connection")
}

async fn handle_remote_v1_http2_request(
    request: HyperRequest<hyper::body::Incoming>,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> std::result::Result<HyperResponse<Full<Bytes>>, std::convert::Infallible> {
    let parsed = match parsed_http2_request(request).await {
        Ok(parsed) => parsed,
        Err(response) => return Ok(hyper_response(response)),
    };
    let response = if request_is_authorized(&parsed, &context) {
        handle_remote_v1_request_with_headers(
            &context,
            &store,
            &parsed.method,
            &parsed.path,
            &parsed.headers,
            parsed.body.as_deref(),
        )
        .unwrap_or_else(|_| error_response(500, "request_failed"))
    } else {
        error_response(401, "auth_failed")
    };
    Ok(hyper_response(response))
}

async fn parsed_http2_request(
    request: HyperRequest<hyper::body::Incoming>,
) -> std::result::Result<request::ParsedHttpRequest, RemoteV1Response> {
    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .map_err(|_| error_response(400, "truncated_body"))?
        .to_bytes()
        .to_vec();
    Ok(request::ParsedHttpRequest {
        method: parts.method.as_str().to_string(),
        path: parts
            .uri
            .path_and_query()
            .map(|value| value.as_str().to_string())
            .unwrap_or_else(|| "/".to_string()),
        headers: http2_headers(&parts.headers),
        authorization: parts
            .headers
            .get(hyper::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        body: if body.is_empty() { None } else { Some(body) },
    })
}

fn http2_headers(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
        })
        .collect()
}

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
