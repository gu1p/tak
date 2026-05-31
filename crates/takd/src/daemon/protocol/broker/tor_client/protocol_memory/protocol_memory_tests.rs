use super::{RemoteProtocol, live_protocol};
use std::time::Duration;

const TTL: Duration = Duration::from_secs(60);

// A fresh HTTP/1.1 pin is honored so a genuinely h1-only peer is not re-dialed
// for h2 on every heartbeat.
#[test]
fn fresh_http1_pin_is_honored() {
    assert_eq!(
        live_protocol(RemoteProtocol::Http1, Duration::from_secs(5), TTL),
        Some(RemoteProtocol::Http1)
    );
}

// The headline guarantee: a transient h2 miss that pinned h1 must lapse so the
// peer is re-probed for HTTP/2 instead of being poisoned forever.
#[test]
fn expired_http1_pin_lapses_so_h2_is_retried() {
    assert_eq!(
        live_protocol(RemoteProtocol::Http1, TTL + Duration::from_secs(1), TTL),
        None
    );
    // Boundary: exactly at the TTL the pin has expired.
    assert_eq!(live_protocol(RemoteProtocol::Http1, TTL, TTL), None);
}

// The positive HTTP/2 memory only confirms the default preference, so it is
// never time-boxed — even long after it was recorded.
#[test]
fn http2_memory_never_expires() {
    assert_eq!(
        live_protocol(RemoteProtocol::Http2, TTL * 1000, TTL),
        Some(RemoteProtocol::Http2)
    );
}
