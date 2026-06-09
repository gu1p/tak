#![cfg(test)]

use super::workspace_upload_cache::{CachedUpload, SharedWorkspaceUploadCache, UploadClaim};
use tak_proto::WorkspaceUploadRef;

fn sample_upload(upload_id: &str) -> CachedUpload {
    CachedUpload {
        upload: WorkspaceUploadRef {
            upload_id: upload_id.to_string(),
            sha256: "0".repeat(64),
            size_bytes: 42,
        },
        preferred_node_id: Some("worker-1".to_string()),
        archive_byte_len: 42,
    }
}

fn key() -> (String, String) {
    ("node-a".to_string(), "content-hash".to_string())
}

#[tokio::test]
async fn lead_then_publish_is_reused() {
    let cache = SharedWorkspaceUploadCache::default();
    assert!(cache.peek(&key()).is_none());
    let UploadClaim::Lead(guard) = cache.claim(key()).await else {
        panic!("first claim on an empty key must lead");
    };
    guard.publish(sample_upload("upload-1"));

    assert_eq!(cache.peek(&key()).unwrap().upload.upload_id, "upload-1");
    match cache.claim(key()).await {
        UploadClaim::Reuse(cached) => {
            assert_eq!(cached.upload.upload_id, "upload-1");
            assert_eq!(cached.preferred_node_id.as_deref(), Some("worker-1"));
        }
        UploadClaim::Lead(_) => panic!("a published key must be reused, not led again"),
    }
}

#[tokio::test]
async fn concurrent_claims_single_flight_to_one_upload() {
    let cache = SharedWorkspaceUploadCache::default();
    let UploadClaim::Lead(guard) = cache.claim(key()).await else {
        panic!("leader expected");
    };
    let follower = {
        let cache = cache.clone();
        tokio::spawn(async move {
            match cache.claim(key()).await {
                UploadClaim::Reuse(cached) => cached.upload.upload_id,
                UploadClaim::Lead(_) => {
                    panic!("follower must not lead while a leader holds the slot")
                }
            }
        })
    };
    tokio::task::yield_now().await;
    guard.publish(sample_upload("leader-upload"));
    assert_eq!(follower.await.unwrap(), "leader-upload");
}

#[tokio::test]
async fn leader_failure_lets_a_waiter_reclaim() {
    let cache = SharedWorkspaceUploadCache::default();
    let UploadClaim::Lead(guard) = cache.claim(key()).await else {
        panic!("leader expected");
    };
    let waiter = {
        let cache = cache.clone();
        tokio::spawn(async move { matches!(cache.claim(key()).await, UploadClaim::Lead(_)) })
    };
    tokio::task::yield_now().await;
    drop(guard); // leader failed without publishing
    assert!(
        waiter.await.unwrap(),
        "a waiter must re-claim leadership after the leader fails"
    );
    assert!(cache.peek(&key()).is_none());
}

#[tokio::test]
async fn invalidate_drops_completed_entry_and_distinct_keys_are_independent() {
    let cache = SharedWorkspaceUploadCache::default();
    let UploadClaim::Lead(guard) = cache.claim(key()).await else {
        panic!("leader expected");
    };
    guard.publish(sample_upload("upload-1"));
    assert!(cache.peek(&key()).is_some());
    cache.invalidate(&key());
    assert!(cache.peek(&key()).is_none());
    assert!(matches!(cache.claim(key()).await, UploadClaim::Lead(_)));

    // A different node id is a different key with its own upload.
    let other = ("node-b".to_string(), "content-hash".to_string());
    assert!(matches!(cache.claim(other).await, UploadClaim::Lead(_)));
}
