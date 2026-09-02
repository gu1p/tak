use takd::AttemptTransport;

use crate::support::v2_remote_output_validation::completed;

#[tokio::test]
async fn origin_rejects_an_undeclared_remote_path_before_importing_its_blob() {
    assert_rejected("//:check", "stolen.txt", "not declared").await;
}

#[tokio::test]
async fn origin_rejects_a_remote_producer_outside_the_dispatched_job_before_import() {
    assert_rejected("//:intruder", "result.txt", "outside the dispatched job").await;
}

#[tokio::test]
async fn origin_requires_the_current_transport_and_fence_before_importing_remote_outputs() {
    let case = completed("//:check", "result.txt").await;
    let mut wrong_fence = case.command.clone();
    wrong_fence.fencing_token.push('x');
    assert!(case.transport.reconcile(&wrong_fence).await.is_err());
    let mut wrong_transport = case.command.clone();
    wrong_transport.transport = Some("tor".into());
    assert!(case.transport.reconcile(&wrong_transport).await.is_err());
    assert!(!case.blob_path.exists());
}

async fn assert_rejected(producer: &str, path: &str, expected: &str) {
    let case = completed(producer, path).await;
    let error = case.transport.reconcile(&case.command).await.unwrap_err();
    assert!(error.to_string().contains(expected), "{error:#}");
    assert!(!case.blob_path.exists(), "invalid output reached the origin CAS");
}
