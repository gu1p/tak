use super::{TorBroker, http1_pin_ttl};

/// What protocol a given peer has proven it speaks, so we stop re-dialing a
/// doomed HTTP/2 connection on every heartbeat once we know a peer only answers
/// HTTP/1.1. Absence means "not yet known; try HTTP/2 first".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::daemon::protocol::broker) enum RemoteProtocol {
    Http2,
    Http1,
}

fn protocol_key(endpoint: &str, node_id: &str) -> String {
    format!("{endpoint}\n{node_id}")
}

// An HTTP/1.1 pin is only a temporary compatibility cache: honor it while fresh,
// but let it lapse so a peer that merely had a transient h2 hiccup (a
// timeout-class failure over Tor) is re-probed for HTTP/2 rather than poisoned
// forever. A genuinely h1-only legacy peer simply re-pins on its next fallback —
// one doomed h2 dial per TTL, not per 15s heartbeat. The positive HTTP/2 memory
// only confirms the default preference, so it never expires.
fn live_protocol(
    protocol: RemoteProtocol,
    age: std::time::Duration,
    ttl: std::time::Duration,
) -> Option<RemoteProtocol> {
    match protocol {
        RemoteProtocol::Http1 if age >= ttl => None,
        protocol => Some(protocol),
    }
}

impl TorBroker {
    pub(in crate::daemon::protocol::broker) async fn remote_protocol(
        &self,
        endpoint: &str,
        node_id: &str,
    ) -> Option<RemoteProtocol> {
        let guard = self.inner.remote_protocols.lock().await;
        let (protocol, pinned_at) = guard.get(&protocol_key(endpoint, node_id))?;
        live_protocol(*protocol, pinned_at.elapsed(), http1_pin_ttl())
    }

    pub(in crate::daemon::protocol::broker) async fn set_remote_protocol(
        &self,
        endpoint: &str,
        node_id: &str,
        protocol: RemoteProtocol,
    ) {
        self.inner.remote_protocols.lock().await.insert(
            protocol_key(endpoint, node_id),
            (protocol, std::time::Instant::now()),
        );
    }

    pub(super) async fn clear_remote_protocol(&self, endpoint: &str, node_id: &str) {
        self.inner
            .remote_protocols
            .lock()
            .await
            .remove(&protocol_key(endpoint, node_id));
    }
}

#[cfg(test)]
mod protocol_memory_tests;
