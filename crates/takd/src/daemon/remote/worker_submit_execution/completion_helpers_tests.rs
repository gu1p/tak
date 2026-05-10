use anyhow::anyhow;

use super::failure_stderr_tail;

#[test]
fn failure_stderr_tail_preserves_error_chain() {
    let error = anyhow!("docker build error: package index fetch failed")
        .context("infra error: container lifecycle build failed");

    let tail = failure_stderr_tail(&error, "");

    assert!(tail.contains("infra error: container lifecycle build failed"));
    assert!(tail.contains("docker build error: package index fetch failed"));
}

#[test]
fn failure_stderr_tail_prepends_error_chain_to_existing_stderr() {
    let error = anyhow!("docker build error: package index fetch failed")
        .context("infra error: container lifecycle build failed");

    let tail = failure_stderr_tail(&error, "existing stderr\n");

    assert!(tail.starts_with("infra error: container lifecycle build failed"));
    assert!(tail.contains("docker build error: package index fetch failed"));
    assert!(tail.ends_with("existing stderr\n"));
}
