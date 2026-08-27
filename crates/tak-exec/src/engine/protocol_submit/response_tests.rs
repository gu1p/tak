use super::submit_status_failure;

#[test]
fn transient_submit_http_statuses_are_retryable() {
    for status in [408, 429, 500, 502, 503, 599] {
        let failure = submit_status_failure(status, "builder-a");

        assert!(failure.is_retryable(), "HTTP {status} should be retryable");
    }
}

#[test]
fn permanent_submit_http_statuses_are_not_retryable() {
    for status in [400, 404] {
        let failure = submit_status_failure(status, "builder-a");

        assert!(
            !failure.is_retryable(),
            "HTTP {status} should not be retryable"
        );
    }
}
