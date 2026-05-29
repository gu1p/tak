use super::TorBroker;

/// What protocol a given peer has proven it speaks, so we stop re-dialing a
/// doomed HTTP/2 connection on every heartbeat once we know a peer only answers
/// HTTP/1.1. Absence means "not yet known; try HTTP/2 first".
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::daemon::protocol::broker) enum RemoteProtocol {
    Http2,
    Http1,
}

fn protocol_key(endpoint: &str, node_id: &str) -> String {
    format!("{endpoint}\n{node_id}")
}

impl TorBroker {
    pub(in crate::daemon::protocol::broker) async fn remote_protocol(
        &self,
        endpoint: &str,
        node_id: &str,
    ) -> Option<RemoteProtocol> {
        self.inner
            .remote_protocols
            .lock()
            .await
            .get(&protocol_key(endpoint, node_id))
            .copied()
    }

    pub(in crate::daemon::protocol::broker) async fn set_remote_protocol(
        &self,
        endpoint: &str,
        node_id: &str,
        protocol: RemoteProtocol,
    ) {
        self.inner
            .remote_protocols
            .lock()
            .await
            .insert(protocol_key(endpoint, node_id), protocol);
    }

    pub(super) async fn clear_remote_protocol(&self, endpoint: &str, node_id: &str) {
        self.inner
            .remote_protocols
            .lock()
            .await
            .remove(&protocol_key(endpoint, node_id));
    }
}
