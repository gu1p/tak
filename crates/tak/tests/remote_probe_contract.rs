#[test]
fn https_authority_defaults_to_443() {
    assert_eq!(
        tak_exec::endpoint_socket_addr("https://remote.example").expect("https authority"),
        "remote.example:443"
    );
}

#[test]
fn http_authority_defaults_to_80() {
    assert_eq!(
        tak_exec::endpoint_socket_addr("http://remote.example").expect("http authority"),
        "remote.example:80"
    );
}

#[test]
fn ipv6_authority_preserves_brackets_and_port() {
    assert_eq!(
        tak_exec::endpoint_socket_addr("https://[::1]").expect("https ipv6 authority"),
        "[::1]:443"
    );
    assert_eq!(
        tak_exec::endpoint_host_port("https://[::1]:8443/path").expect("ipv6 host port"),
        ("::1".to_string(), 8443)
    );
}

#[test]
fn bare_authority_with_port_and_userinfo_normalizes_correctly() {
    assert_eq!(
        tak_exec::endpoint_socket_addr("builder.example:9000").expect("bare authority"),
        "builder.example:9000"
    );
    assert_eq!(
        tak_exec::endpoint_socket_addr("https://user:pass@[::1]:9443/path?x=1#frag")
            .expect("userinfo authority"),
        "[::1]:9443"
    );
}

#[test]
fn socket_addr_from_host_port_wraps_ipv6_hosts() {
    assert_eq!(
        tak_exec::socket_addr_from_host_port("builder.example", 9000),
        "builder.example:9000"
    );
    assert_eq!(
        tak_exec::socket_addr_from_host_port("127.0.0.1", 9000),
        "127.0.0.1:9000"
    );
    assert_eq!(
        tak_exec::socket_addr_from_host_port("::1", 9000),
        "[::1]:9000"
    );
}
