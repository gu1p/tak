use super::*;

fn strict_remote_target(kind: RemoteTransportKind, endpoint: &str) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "node-a".to_string(),
        endpoint: endpoint.to_string(),
        transport_kind: kind,
        service_auth_env: None,
        runtime: None,
    }
}

#[test]
fn transport_factory_selects_direct_transport_variant() {
    assert_eq!(
        TransportFactory::transport_name(RemoteTransportKind::DirectHttps),
        "direct"
    );
}

#[test]
fn transport_factory_selects_tor_transport_variant() {
    assert_eq!(
        TransportFactory::transport_name(RemoteTransportKind::Tor),
        "tor"
    );
}

#[test]
fn transport_factory_resolves_socket_addr_for_supported_transports() {
    for kind in [RemoteTransportKind::DirectHttps, RemoteTransportKind::Tor] {
        let target = strict_remote_target(kind, "http://127.0.0.1:4242");
        let socket_addr = TransportFactory::socket_addr(&target)
            .expect("socket address should resolve for supported transport");
        assert_eq!(socket_addr, "127.0.0.1:4242");
    }
}

#[test]
fn endpoint_socket_addr_defaults_port_by_scheme_when_missing() {
    let https = strict_remote_target(RemoteTransportKind::DirectHttps, "https://build.internal");
    let tor_http = strict_remote_target(
        RemoteTransportKind::Tor,
        "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion",
    );

    assert_eq!(
        TransportFactory::socket_addr(&https).expect("https without explicit port"),
        "build.internal:443"
    );
    assert_eq!(
        TransportFactory::socket_addr(&tor_http).expect("onion http without explicit port"),
        "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion:80"
    );
}

#[test]
fn endpoint_socket_addr_accepts_full_url_forms_without_explicit_port() {
    let direct_full_url = strict_remote_target(
        RemoteTransportKind::DirectHttps,
        "https://build.internal?region=us-east#ignored",
    );
    let tor_full_url = strict_remote_target(
        RemoteTransportKind::Tor,
        "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion?queue=default#anchor",
    );

    assert_eq!(
        TransportFactory::socket_addr(&direct_full_url).expect("direct full URL"),
        "build.internal:443"
    );
    assert_eq!(
        TransportFactory::socket_addr(&tor_full_url).expect("tor full URL"),
        "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion:80"
    );
}

#[test]
fn transport_variant_branching_isolated_to_transport_factory() {
    let source = include_str!("lib.rs");
    let production = source.split("\n#[cfg(test)]").next().unwrap_or(source);
    let sites = production
        .lines()
        .filter(|line| line.contains("RemoteTransportKind::"))
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        sites,
        vec![
            "RemoteTransportKind::DirectHttps => &DIRECT_HTTPS_TRANSPORT_ADAPTER,".to_string(),
            "RemoteTransportKind::Tor => &TOR_TRANSPORT_ADAPTER,".to_string(),
        ],
        "transport variant branching must remain isolated to TransportFactory::adapter"
    );
}
