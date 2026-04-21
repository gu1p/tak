#![cfg(test)]

use anyhow::anyhow;

use super::{RemoteSubmitFailure, RemoteSubmitFailureKind, is_auth_submit_failure};

#[test]
fn auth_submit_fallback_requires_typed_submit_classification() {
    assert!(!is_auth_submit_failure(&anyhow!(
        "infra error: remote node builder-a auth failed during submit with HTTP 401"
    )));

    assert!(is_auth_submit_failure(
        &RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Auth,
            message: "infra error: remote node builder-a auth failed during submit with HTTP 401"
                .into(),
        }
        .into()
    ));
}
