use super::TorBroker;
use super::remote_exchange;

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
        let auth = remote_exchange::authorization_value(bearer_token).unwrap_or_default();
        let session_key = format!("{endpoint}\n{node_id}\n{auth}");
        self.evict_http2_session(&session_key).await;
        self.clear_remote_protocol(endpoint, node_id).await;
    }
}
