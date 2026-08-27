#![cfg(test)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::http_server_test_support::node_context;
use super::runtime_services::supervise_runtime_services;

#[test]
fn a_remote_context_claims_its_runtime_services_once() {
    let context = node_context();

    assert!(context.claim_remote_runtime_services());
    assert!(!context.claim_remote_runtime_services());
}

#[tokio::test]
async fn runtime_service_supervisor_restarts_an_exited_service_generation() {
    let starts = Arc::new(AtomicUsize::new(0));
    let observed = starts.clone();
    let supervisor = tokio::spawn(supervise_runtime_services(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        vec![tokio::spawn(async {})]
    }));

    tokio::time::timeout(Duration::from_secs(3), async {
        while starts.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("supervisor should replace an exited service generation");
    supervisor.abort();
}
