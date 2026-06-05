#![cfg(test)]

use anyhow::anyhow;

use crate::engine::preflight_fallback::is_auth_submit_failure;
use crate::engine::remote_submit_failure::RemoteSubmitFailure;

#[test]
fn auth_submit_fallback_requires_typed_submit_classification() {
    assert!(!is_auth_submit_failure(&anyhow!(
        "infra error: remote node builder-a auth failed during submit with HTTP 401"
    )));

    let err = RemoteSubmitFailure::auth(
        "infra error: remote node builder-a auth failed during submit with HTTP 401".into(),
    );
    assert!(is_auth_submit_failure(&err.into()));
}
