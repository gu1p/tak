use super::remote_headers;

#[test]
fn explicit_worker_v2_header_remains_unique() {
    let headers = remote_headers(
        "worker-a",
        "secret",
        &[("X-Tak-Protocol-Version".into(), "v2".into())],
    );
    let versions = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("X-Tak-Protocol-Version"))
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();

    assert_eq!(versions, vec!["v2"]);
}

#[test]
fn missing_protocol_input_defaults_to_exactly_v2() {
    let headers = remote_headers("worker-a", "secret", &[]);
    let versions = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("X-Tak-Protocol-Version"))
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();

    assert_eq!(versions, vec!["v2"]);
}
