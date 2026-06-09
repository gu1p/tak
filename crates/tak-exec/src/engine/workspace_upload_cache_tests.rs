use super::{CachedUpload, SharedWorkspaceUploadCache, UploadClaim};
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
    let key = key();
    assert!(cache.peek(&key).is_none());

    let UploadClaim::Lead(guard) = cache.claim(key.clone()).await else {
        panic!("first claim on an empty key must lead");
    };
    guard.publish(sample_upload("upload-1"));

    assert_eq!(cache.peek(&key).unwrap().upload.upload_id, "upload-1");
    match cache.claim(key.clone()).await {
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
    let key = key();

    let UploadClaim::Lead(guard) = cache.claim(key.clone()).await else {
        panic!("leader expected");
    };

    // A second, concurrent claim for the same key must wait for the leader and reuse its
    // result rather than performing its own upload.
    let follower = {
        let cache = cache.clone();
        let key = key.clone();
        tokio::spawn(async move {
            match cache.claim(key).await {
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
    let key = key();

    let UploadClaim::Lead(guard) = cache.claim(key.clone()).await else {
        panic!("leader expected");
    };

    let waiter = {
        let cache = cache.clone();
        let key = key.clone();
        tokio::spawn(async move { matches!(cache.claim(key).await, UploadClaim::Lead(_)) })
    };

    tokio::task::yield_now().await;
    drop(guard); // leader failed without publishing

    assert!(
        waiter.await.unwrap(),
        "a waiter must be able to re-claim leadership after the leader fails"
    );
    // The waiter's transient lead guard was dropped unpublished, so the slot is empty again.
    assert!(cache.peek(&key).is_none());
}

#[tokio::test]
async fn invalidate_drops_completed_entry() {
    let cache = SharedWorkspaceUploadCache::default();
    let key = key();

    let UploadClaim::Lead(guard) = cache.claim(key.clone()).await else {
        panic!("leader expected");
    };
    guard.publish(sample_upload("upload-1"));
    assert!(cache.peek(&key).is_some());

    cache.invalidate(&key);
    assert!(cache.peek(&key).is_none());

    // After invalidation the next claim leads a fresh upload.
    assert!(matches!(cache.claim(key).await, UploadClaim::Lead(_)));
}

#[tokio::test]
async fn distinct_keys_are_independent() {
    let cache = SharedWorkspaceUploadCache::default();
    let UploadClaim::Lead(guard_a) = cache.claim(("node-a".into(), "h".into())).await else {
        panic!("lead a");
    };
    // A different node id is a different key and must lead its own upload.
    assert!(matches!(
        cache.claim(("node-b".to_string(), "h".to_string())).await,
        UploadClaim::Lead(_)
    ));
    guard_a.publish(sample_upload("a"));
    assert_eq!(
        cache
            .peek(&("node-a".into(), "h".into()))
            .unwrap()
            .upload
            .upload_id,
        "a"
    );
}
