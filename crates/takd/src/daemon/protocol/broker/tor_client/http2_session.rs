use super::*;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};

pub(in crate::daemon::protocol::broker) type BrokerBody = BoxBody<Bytes, std::io::Error>;

// Client-side HTTP/2 receive windows. These govern how much *response* body a
// peer may stream to us before waiting on our WINDOW_UPDATE; the default 64 KiB
// would throttle a multi-MB result fetch to one window per onion round trip. The
// matching upload (request-body) limit lives on the server (see the remote-v1
// HTTP/2 server builder). Connection window covers the aggregate of concurrent
// streams on one pooled session.
const HTTP2_STREAM_WINDOW: u32 = 4 * 1024 * 1024;
const HTTP2_CONNECTION_WINDOW: u32 = 8 * 1024 * 1024;
// Keep a pooled connection permanently warm: hyper sends an HTTP/2 PING every
// interval even with no active streams, so an idle onion link is not closed
// between submits, and a dead peer fails the PING within the timeout so the
// connection (and `is_closed`) flips and the keeper can redial. The timeout is
// generous because onion round trips are slow; a false close just costs a redial.
const HTTP2_KEEP_ALIVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
const HTTP2_KEEP_ALIVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub(super) struct Http2Session {
    sender: SendRequest<BrokerBody>,
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
            .initial_connection_window_size(HTTP2_CONNECTION_WINDOW)
            // A timer is required for keep-alive PINGs to be scheduled (hyper
            // panics otherwise).
            .timer(TokioTimer::new())
            .keep_alive_interval(HTTP2_KEEP_ALIVE_INTERVAL)
            .keep_alive_timeout(HTTP2_KEEP_ALIVE_TIMEOUT)
            .keep_alive_while_idle(true);
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

    // Whether the underlying HTTP/2 connection has closed (peer dropped or
    // keep-alive timed out). The connection keeper uses this to redial a dead
    // pooled session, and the request path treats a closed session as a miss.
    pub(super) fn is_closed(&self) -> bool {
        self.sender.is_closed()
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

    pub(super) async fn send_stream(
        &self,
        request: BrokerHttp2StreamRequest,
    ) -> std::result::Result<BrokerHttp2Response, BrokerHttpError> {
        let request = request.into_hyper_request()?;
        let mut sender = self.sender.clone();
        let response = sender.send_request(request).await.map_err(|err| {
            tracing::debug!(error = %err, is_closed = sender.is_closed(), "h2 stream send_request failed");
            BrokerHttpError::bad_gateway("http2_request_failed", err)
        })?;
        BrokerHttp2Response::from_hyper(response).await
    }
}
