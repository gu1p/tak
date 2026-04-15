#![cfg(test)]

use super::{DirectBaseUrlError, parse_direct_base_url};

#[test]
fn direct_base_url_requires_an_explicit_port() {
    for base_url in [
        "http://127.0.0.1",
        "https://builder.example",
        "http://[::1]",
    ] {
        assert_eq!(
            parse_direct_base_url(Some(base_url)),
            Err(DirectBaseUrlError::MissingPort),
            "expected missing explicit port for {base_url}"
        );
    }
}

#[test]
fn direct_base_url_accepts_explicit_ports_including_ipv6_and_zero() {
    assert!(parse_direct_base_url(Some("http://127.0.0.1:8080")).is_ok());
    assert!(parse_direct_base_url(Some("https://[::1]:9443")).is_ok());
    assert!(parse_direct_base_url(Some("http://127.0.0.1:0")).is_ok());
}

#[test]
fn direct_base_url_canonicalizes_mixed_case_http_scheme() {
    let parsed =
        parse_direct_base_url(Some("HTTP://127.0.0.1:43123")).expect("parse mixed-case http");
    assert_eq!(parsed.canonical_base_url(), "http://127.0.0.1:43123");
}
