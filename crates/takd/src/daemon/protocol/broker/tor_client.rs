use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use arti_client::TorClient;

use super::*;

mod config;
mod connect;
mod default;
mod http2_session;
mod protobuf;
mod protocol_memory;
mod remote_exchange;
mod session_pool;
mod shared_client;
mod types;

use config::{
    create_bootstrapped, default_client_tor_config, http1_pin_ttl, http2_handshake_timeout,
    socket_addr_from_host_port, test_tor_onion_dial_addr, tor_connect_retry_delay,
    tor_connect_timeout,
};
use http2_session::Http2Session;
use protocol_memory::RemoteProtocol;
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
    // Session keys with a keeper dial in flight, so a down peer never piles up
    // one (up to 120s) dial loop per 1s tick — only one at a time. A std Mutex so
    // the RAII dial guard can release it on drop (panic/cancel safe).
    http2_dials: Mutex<HashSet<String>>,
    remote_protocols: tokio::sync::Mutex<HashMap<String, (RemoteProtocol, std::time::Instant)>>,
    test_dial_addr: Option<String>,
    state_root: Option<PathBuf>,
    bootstrap_count: AtomicUsize,
    // When set, the broker dials peers through the hidden service's Tor client
    // instead of bootstrapping its own. `requires_shared_client` marks the
    // `tor` transport, where that client is mandatory (we never spin up a rival).
    shared_tor_client: Mutex<Option<TorClient<tor_rtcompat::PreferredRuntime>>>,
    requires_shared_client: bool,
}

enum BrokerClient {
    Arti(Box<TorClient<tor_rtcompat::PreferredRuntime>>),
    Test(String),
}

impl TorBroker {
    pub fn new() -> Self {
        Self::with_options(test_tor_onion_dial_addr(), None, false)
    }

    pub fn for_test_dial_addr(dial_addr: String) -> Self {
        Self::with_options(Some(dial_addr), None, false)
    }

    pub fn for_state_root(state_root: PathBuf) -> Self {
        Self::with_options(test_tor_onion_dial_addr(), Some(state_root), false)
    }

    pub(in crate::daemon) fn with_options(
        test_dial_addr: Option<String>,
        state_root: Option<PathBuf>,
        requires_shared_client: bool,
    ) -> Self {
        Self {
            inner: Arc::new(TorBrokerInner {
                client: tokio::sync::OnceCell::const_new(),
                http2_sessions: tokio::sync::Mutex::new(HashMap::new()),
                http2_dials: Mutex::new(HashSet::new()),
                remote_protocols: tokio::sync::Mutex::new(HashMap::new()),
                test_dial_addr,
                state_root,
                bootstrap_count: AtomicUsize::new(0),
                shared_tor_client: Mutex::new(None),
                requires_shared_client,
            }),
        }
    }

    pub fn bootstrap_count(&self) -> usize {
        self.inner.bootstrap_count.load(Ordering::SeqCst)
    }

    pub async fn warm(&self) -> Result<()> {
        // In shared mode the hidden-service client owns Tor and is bootstrapped
        // by the onion-serving path; the broker borrows it rather than warming
        // (and locking) a second Arti client of its own.
        if self.inner.requires_shared_client {
            return Ok(());
        }
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
        connect::broker_connect(self, endpoint).await
    }

    pub(super) async fn http2_exchange(
        &self,
        endpoint: &str,
        request: BrokerHttp2Request,
    ) -> std::result::Result<BrokerHttp2Response, BrokerHttpError> {
        let session_key = request.session_key(endpoint);
        let (session, reused) = self.http2_session(endpoint, &session_key).await?;
        match session.send(request.clone()).await {
            Ok(response) => Ok(response),
            Err(first) => {
                // Identity-aware: only drop the session that actually failed, not
                // a fresh one a concurrent keeper/request dial may have inserted.
                self.evict_session_if_unchanged(&session_key, &session)
                    .await;
                // Reconnect+retry only a *reused* pooled session (its underlying
                // onion stream may have dropped while idle); a freshly dialed
                // session that already failed surfaces the error to the caller —
                // no second doomed onion dial.
                if reused && request.can_retry_after_failure() {
                    // Surface the *fresh* attempt's failure code, not the stale
                    // pooled-session error: the caller pins HTTP/1.1 only on
                    // `http2_request_failed`, so a transient reconnect/handshake
                    // failure here must not masquerade as that and poison the peer.
                    let (session, _) = self.http2_session(endpoint, &session_key).await?;
                    session.send(request).await
                } else {
                    Err(first)
                }
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
}
