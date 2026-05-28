use super::*;

use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::{TokioExecutor, TokioIo};

const HTTP2_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct Http2Session {
    sender: SendRequest<Full<Bytes>>,
}

impl Http2Session {
    pub(super) async fn connect(
        broker: &TorBroker,
        endpoint: &str,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let stream = broker
            .connect(endpoint)
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("connect_failed", err))?;
        let handshake =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream));
        let (sender, connection) = tokio::time::timeout(HTTP2_HANDSHAKE_TIMEOUT, handshake)
            .await
            .map_err(|_| BrokerHttpError::bad_gateway("http2_unavailable", "handshake timeout"))?
            .map_err(|err| BrokerHttpError::bad_gateway("http2_unavailable", err))?;
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
        let response = sender
            .send_request(request)
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("http2_request_failed", err))?;
        BrokerHttp2Response::from_hyper(response).await
    }
}
