use super::*;

use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::{TokioExecutor, TokioIo};

// Client-side HTTP/2 receive windows. These govern how much *response* body a
// peer may stream to us before waiting on our WINDOW_UPDATE; the default 64 KiB
// would throttle a multi-MB result fetch to one window per onion round trip. The
// matching upload (request-body) limit lives on the server (see the remote-v1
// HTTP/2 server builder). Connection window covers the aggregate of concurrent
// streams on one pooled session.
const HTTP2_STREAM_WINDOW: u32 = 4 * 1024 * 1024;
const HTTP2_CONNECTION_WINDOW: u32 = 8 * 1024 * 1024;

pub(super) struct Http2Session {
    sender: SendRequest<Full<Bytes>>,
}

impl Http2Session {
    pub(super) async fn connect(
        broker: &TorBroker,
        endpoint: &str,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let dial_started = tokio::time::Instant::now();
        let stream = broker
            .connect(endpoint)
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("connect_failed", err))?;
        let dial_ms = dial_started.elapsed().as_millis();
        let mut builder = hyper::client::conn::http2::Builder::new(TokioExecutor::new());
        builder
            .initial_stream_window_size(HTTP2_STREAM_WINDOW)
            .initial_connection_window_size(HTTP2_CONNECTION_WINDOW);
        let handshake = builder.handshake(TokioIo::new(stream));
        let handshake_started = tokio::time::Instant::now();
        let (sender, connection) =
            match tokio::time::timeout(http2_handshake_timeout(), handshake).await {
                Err(_) => {
                    tracing::warn!(
                        endpoint,
                        dial_ms,
                        handshake_ms = handshake_started.elapsed().as_millis(),
                        timeout_ms = http2_handshake_timeout().as_millis(),
                        "h2 handshake timed out"
                    );
                    return Err(BrokerHttpError::bad_gateway(
                        "http2_unavailable",
                        "handshake timeout",
                    ));
                }
                Ok(Err(err)) => {
                    tracing::warn!(endpoint, dial_ms, error = %err, "h2 handshake failed");
                    return Err(BrokerHttpError::bad_gateway("http2_unavailable", err));
                }
                Ok(Ok(pair)) => pair,
            };
        tracing::debug!(
            endpoint,
            dial_ms,
            handshake_ms = handshake_started.elapsed().as_millis(),
            "h2 handshake ok"
        );
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Self { sender })
    }

    pub(super) async fn send(
        &self,
        request: BrokerHttp2Request,
    ) -> std::result::Result<BrokerHttp2Response, BrokerHttpError> {
        let request = request.to_hyper_request()?;
        let mut sender = self.sender.clone();
        let response = sender.send_request(request).await.map_err(|err| {
            tracing::debug!(error = %err, is_closed = sender.is_closed(), "h2 send_request failed");
            BrokerHttpError::bad_gateway("http2_request_failed", err)
        })?;
        BrokerHttp2Response::from_hyper(response).await
    }
}
