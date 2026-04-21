//! Contract test for embedded Tor transport implementation details.

#[test]
fn tor_transport_uses_embedded_arti_client_not_external_socks_proxy() {
    let source = format!(
        "{}\n{}",
        include_str!("../src/engine/mod.rs"),
        include_str!("../src/engine/transport.rs"),
    );
    assert!(
        source.contains("arti_client::TorClient"),
        "tor transport must embed Arti client in-process"
    );
    assert!(
        !source.contains("TAK_TOR_SOCKS5_ADDR"),
        "tor transport must not depend on external socks proxy environment variables"
    );
}
