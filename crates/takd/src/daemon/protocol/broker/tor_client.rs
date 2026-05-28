use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::Result;
use arti_client::TorClient;

use super::*;

mod config;
mod connect;
mod default;
mod http2_session;
mod protobuf;
mod remote_exchange;
mod types;

use config::{
    create_bootstrapped, default_client_tor_config, socket_addr_from_host_port,
    test_tor_onion_dial_addr, tor_connect_retry_delay, tor_connect_timeout,
};
use connect::{connect_tcp, retry_connect};
use http2_session::Http2Session;
pub use types::BrokerForwardResponse;
pub(in crate::daemon::protocol) use types::BrokerRemoteHttpRequest;
pub(super) use types::BrokerRemoteStream;

const DEFAULT_TOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct TorBroker {
    inner: Arc<TorBrokerInner>,
}

struct TorBrokerInner {
    client: tokio::sync::OnceCell<BrokerClient>,
    http2_sessions: tokio::sync::Mutex<HashMap<String, Arc<Http2Session>>>,
    test_dial_addr: Option<String>,
    state_root: Option<PathBuf>,
    bootstrap_count: AtomicUsize,
}

enum BrokerClient {
    Arti(Box<TorClient<tor_rtcompat::PreferredRuntime>>),
    Test(String),
}

impl TorBroker {
    pub fn new() -> Self {
        Self::with_options(test_tor_onion_dial_addr(), None)
    }

    pub fn for_test_dial_addr(dial_addr: String) -> Self {
        Self::with_options(Some(dial_addr), None)
    }

    pub fn for_state_root(state_root: PathBuf) -> Self {
        Self::with_options(test_tor_onion_dial_addr(), Some(state_root))
    }

    fn with_options(test_dial_addr: Option<String>, state_root: Option<PathBuf>) -> Self {
        Self {
            inner: Arc::new(TorBrokerInner {
                client: tokio::sync::OnceCell::const_new(),
                http2_sessions: tokio::sync::Mutex::new(HashMap::new()),
                test_dial_addr,
                state_root,
                bootstrap_count: AtomicUsize::new(0),
            }),
        }
    }

    pub fn bootstrap_count(&self) -> usize {
        self.inner.bootstrap_count.load(Ordering::SeqCst)
    }

    pub async fn warm(&self) -> Result<()> {
        let _ = self.client().await?;
        Ok(())
    }

    pub async fn get_protobuf(
        &self,
        endpoint: &str,
        node_id: &str,
        path: &str,
        bearer_token: &str,
    ) -> Result<(u16, Vec<u8>)> {
        protobuf::get_protobuf(self, endpoint, node_id, path, bearer_token).await
    }

    pub(in crate::daemon::protocol) async fn remote_http_exchange(
        &self,
        remote_request: BrokerRemoteHttpRequest<'_>,
    ) -> Result<BrokerForwardResponse> {
        remote_exchange::remote_http_exchange(self, remote_request).await
    }

    pub(super) async fn connect(&self, endpoint: &str) -> Result<BrokerRemoteStream> {
        let (host, port) = tak_core::endpoint::endpoint_host_port(endpoint)?;
        if !host.ends_with(".onion") {
            return connect_tcp(&socket_addr_from_host_port(&host, port)).await;
        }
        let client = self.client().await?;
        retry_connect(client, &host, port).await
    }

    pub(super) async fn http2_exchange(
        &self,
        endpoint: &str,
        request: BrokerHttp2Request,
    ) -> std::result::Result<BrokerHttp2Response, BrokerHttpError> {
        let session_key = request.session_key(endpoint);
        match self
            .try_http2_exchange(endpoint, &session_key, request.clone())
            .await
        {
            Ok(response) => Ok(response),
            Err(first) => {
                self.evict_http2_session(&session_key).await;
                if !request.can_retry_after_failure() {
                    return Err(first);
                }
                self.try_http2_exchange(endpoint, &session_key, request)
                    .await
                    .map_err(|_| first)
            }
        }
    }

    async fn client(&self) -> Result<&BrokerClient> {
        self.inner
            .client
            .get_or_try_init(|| async {
                self.inner.bootstrap_count.fetch_add(1, Ordering::SeqCst);
                if let Some(dial_addr) = self.inner.test_dial_addr.clone() {
                    return Ok(BrokerClient::Test(dial_addr));
                }
                tak_core::crypto_provider::ensure_rustls_crypto_provider();
                let config = default_client_tor_config(self.inner.state_root.clone())?;
                Ok(BrokerClient::Arti(Box::new(
                    create_bootstrapped(config).await?,
                )))
            })
            .await
    }

    async fn try_http2_exchange(
        &self,
        endpoint: &str,
        session_key: &str,
        request: BrokerHttp2Request,
    ) -> std::result::Result<BrokerHttp2Response, BrokerHttpError> {
        let session = self.http2_session(endpoint, session_key).await?;
        session.send(request).await
    }

    async fn http2_session(
        &self,
        endpoint: &str,
        session_key: &str,
    ) -> std::result::Result<Arc<Http2Session>, BrokerHttpError> {
        if let Some(session) = self
            .inner
            .http2_sessions
            .lock()
            .await
            .get(session_key)
            .cloned()
        {
            return Ok(session);
        }
        let session = Arc::new(Http2Session::connect(self, endpoint).await?);
        self.inner
            .http2_sessions
            .lock()
            .await
            .insert(session_key.to_string(), Arc::clone(&session));
        Ok(session)
    }

    async fn evict_http2_session(&self, session_key: &str) {
        self.inner.http2_sessions.lock().await.remove(session_key);
    }

    pub async fn evict_http2_session_for_peer(
        &self,
        endpoint: &str,
        node_id: &str,
        bearer_token: &str,
    ) {
        let auth = remote_exchange::authorization_value(bearer_token).unwrap_or_default();
        let session_key = format!("{endpoint}\n{node_id}\n{auth}");
        self.evict_http2_session(&session_key).await;
    }
}
