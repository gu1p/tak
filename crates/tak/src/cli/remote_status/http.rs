use anyhow::{Context, anyhow};
use prost::Message;
use tak_proto::NodeStatusResponse;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cli::remote_probe_support::ProbeAttemptError;
use crate::cli::remote_probe_support::http::send_http_get;

pub(super) async fn fetch_status_once<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeStatusResponse, ProbeAttemptError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (status, body) = send_http_get(
        stream,
        authority,
        "/v1/node/status",
        bearer_token,
        base_url,
        "write node status",
    )
    .await?;
    if status != 200 {
        return Err(ProbeAttemptError::final_error(anyhow!(
            "node status failed with HTTP {status}"
        )));
    }
    NodeStatusResponse::decode(body.as_slice())
        .context("decode node status protobuf")
        .map_err(ProbeAttemptError::final_error)
}

#[path = "http_malformed_response_tests.rs"]
mod http_malformed_response_tests;
#[path = "http_tests.rs"]
mod http_tests;
