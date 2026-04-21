#![cfg(test)]

use anyhow::anyhow;

use super::token_state::{ReadTokenError, should_retry_token_error};

#[test]
fn only_typed_not_ready_token_errors_retry() {
    assert!(should_retry_token_error(&ReadTokenError::NotReady));
    assert!(!should_retry_token_error(&ReadTokenError::Invalid(
        anyhow!("agent token not ready because parsing failed")
    )));
}
