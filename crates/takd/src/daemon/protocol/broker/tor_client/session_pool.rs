use std::collections::HashSet;
use std::sync::Arc;

use super::http2_session::Http2Session;
use super::{BrokerHttpError, RemoteProtocol, TorBroker, remote_exchange};

impl TorBroker {
    pub(super) async fn evict_http2_session(&self, session_key: &str) {
        self.inner.http2_sessions.lock().await.remove(session_key);
    }

    pub async fn evict_http2_session_for_peer(
        &self,
        endpoint: &str,
        node_id: &str,
        bearer_token: &str,
    ) {
        let session_key = session_key(endpoint, node_id, bearer_token);
        self.evict_http2_session(&session_key).await;
        self.clear_remote_protocol(endpoint, node_id).await;
    }

    // Returns a pooled HTTP/2 session, reconnecting if none is cached or the
    // cached one has closed; the bool reports whether it came from the pool.
    pub(super) async fn http2_session(
        &self,
        endpoint: &str,
        session_key: &str,
    ) -> std::result::Result<(Arc<Http2Session>, bool), BrokerHttpError> {
        // Clone the cached Arc and drop the lock before any await/redial — the
        // tokio Mutex must never be held across the onion dial.
        let cached = self
            .inner
            .http2_sessions
            .lock()
            .await
            .get(session_key)
            .cloned();
        if let Some(session) = cached {
            // A closed pooled session (peer dropped or keep-alive timed out) is a
            // miss: never hand a dead connection to a request.
            if !session.is_closed() {
                return Ok((session, true));
            }
            self.evict_session_if_unchanged(session_key, &session).await;
        }
        let session = Arc::new(Http2Session::connect(self, endpoint).await?);
        self.inner
            .http2_sessions
            .lock()
            .await
            .insert(session_key.to_string(), Arc::clone(&session));
        Ok((session, false))
    }

    // Evict only if the pooled entry is still the exact session we saw, so a
    // fresh session inserted by a concurrent dialer is never dropped.
    pub(super) async fn evict_session_if_unchanged(
        &self,
        session_key: &str,
        session: &Arc<Http2Session>,
    ) {
        let mut sessions = self.inner.http2_sessions.lock().await;
        if sessions
            .get(session_key)
            .is_some_and(|current| Arc::ptr_eq(current, session))
        {
            sessions.remove(session_key);
        }
    }

    // Eagerly establish (or redial) the permanently-warm pooled connection for a
    // peer. The connection keeper calls this for every configured peer so a
    // submit always lands on an already-open connection rather than cold-dialing.
    pub(in crate::daemon) async fn ensure_warm_session(
        &self,
        endpoint: &str,
        node_id: &str,
        bearer_token: &str,
    ) -> anyhow::Result<()> {
        // A peer known to speak only HTTP/1.1 has no pooled connection to hold; do
        // not hammer it with a doomed h2 dial every tick (mirrors the heartbeat).
        if self.remote_protocol(endpoint, node_id).await == Some(RemoteProtocol::Http1) {
            return Ok(());
        }
        let session_key = session_key(endpoint, node_id, bearer_token);
        // Single-flight keeper dials. The guard releases on drop, so a panic or a
        // cancelled task can never leave a peer permanently un-warmable.
        let Some(_dial) = self.claim_http2_dial(&session_key) else {
            return Ok(());
        };
        self.http2_session(endpoint, &session_key)
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from)
    }

    fn claim_http2_dial(&self, session_key: &str) -> Option<DialGuard<'_>> {
        let claimed = self
            .inner
            .http2_dials
            .lock()
            .expect("http2_dials lock poisoned")
            .insert(session_key.to_string());
        claimed.then(|| DialGuard {
            broker: self,
            session_key: session_key.to_string(),
        })
    }

    // Drop pooled sessions whose peer is no longer configured, so a connection to
    // a removed peer (possibly resurrected by an in-flight dial) cannot linger.
    pub(in crate::daemon) async fn retain_warm_sessions(&self, live_keys: &HashSet<String>) {
        self.inner
            .http2_sessions
            .lock()
            .await
            .retain(|key, _| live_keys.contains(key));
    }

    pub(in crate::daemon) fn warm_session_key(
        &self,
        endpoint: &str,
        node_id: &str,
        bearer_token: &str,
    ) -> String {
        session_key(endpoint, node_id, bearer_token)
    }
}

// Releases its session key from the in-flight dial set when dropped, so the key
// is freed even if the dial future panics or is cancelled.
struct DialGuard<'a> {
    broker: &'a TorBroker,
    session_key: String,
}

impl Drop for DialGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut dials) = self.broker.inner.http2_dials.lock() {
            dials.remove(&self.session_key);
        }
    }
}

fn session_key(endpoint: &str, node_id: &str, bearer_token: &str) -> String {
    let auth = remote_exchange::authorization_value(bearer_token).unwrap_or_default();
    format!("{endpoint}\n{node_id}\n{auth}")
}
