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
