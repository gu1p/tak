//! Contract test for local Tor broker transport implementation details.

#[test]
fn tor_transport_uses_local_takd_broker_not_external_socks_proxy() {
    let source = format!(
        "{}\n{}",
        include_str!("../src/engine/mod.rs"),
        include_str!("../src/engine/transport.rs"),
    );
    assert!(
        source.contains("TAKD_SOCKET") && source.contains("takd Tor broker"),
        "tor transport must route onion remotes through the local takd broker"
    );
    assert!(
        !source.contains("TAK_TOR_SOCKS5_ADDR"),
        "tor transport must not depend on external socks proxy environment variables"
    );
}

#[test]
fn tak_run_transport_does_not_bootstrap_client_side_tor() {
    let source = format!(
        "{}\n{}",
        include_str!("../src/engine/transport.rs"),
        include_str!("../src/engine/transport_tor.rs"),
    );
    assert!(
        !source.contains("connect_in_process_tor"),
        "tak run must not keep a client-side Tor fallback"
    );
    assert!(
        !source.contains("TorClient::create_bootstrapped"),
        "tak run must not bootstrap Arti directly"
    );
}

#[test]
fn tor_run_does_not_send_endpoint_forwarding_headers() {
    let source = format!(
        "{}\n{}",
        include_str!("../src/engine/protocol_result_http/request.rs"),
        include_str!("../src/engine/transport.rs"),
    );
    assert!(
        !source.contains("X-Tak-Remote-Endpoint"),
        "Tor execution must let local takd resolve peer endpoints from daemon state"
    );
}
