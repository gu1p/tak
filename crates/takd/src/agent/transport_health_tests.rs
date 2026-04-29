#![cfg(test)]

use std::fs;

use super::{TransportState, read_transport_health};

#[test]
fn empty_transport_health_file_reports_recovering_detail() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("transport-health.toml"), "").expect("write empty health");

    let health = read_transport_health(temp.path())
        .expect("empty health should not be fatal")
        .expect("empty health should produce unready state");

    assert_eq!(health.transport_state, TransportState::Recovering);
    assert!(health.base_url.is_none());
    assert!(
        health
            .detail
            .expect("detail")
            .contains("transport health file is unreadable")
    );
}

#[test]
fn malformed_transport_health_file_reports_recovering_detail() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("transport-health.toml"), "base_url = 42")
        .expect("write malformed health");

    let health = read_transport_health(temp.path())
        .expect("malformed health should not be fatal")
        .expect("malformed health should produce unready state");

    assert_eq!(health.transport_state, TransportState::Recovering);
    assert!(
        health
            .detail
            .expect("detail")
            .contains("transport health file is unreadable")
    );
}
