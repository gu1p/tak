#![cfg(test)]
use super::{FetchOutcome, classify_fetch_status};

#[test]
fn classify_fetch_status_maps_expected_outcomes() {
    assert_eq!(classify_fetch_status(200), FetchOutcome::Ok);
    assert_eq!(classify_fetch_status(404), FetchOutcome::NotFound);
    for status in [408, 429, 500, 502, 503, 599] {
        assert_eq!(
            classify_fetch_status(status),
            FetchOutcome::Retryable,
            "status {status} should be retryable"
        );
    }
    for status in [400, 401, 403, 418, 451, 301] {
        assert_eq!(
            classify_fetch_status(status),
            FetchOutcome::Terminal,
            "status {status} should be terminal"
        );
    }
}
