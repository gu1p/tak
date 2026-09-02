use super::{RemoteImageCacheRuntimeConfig, RemoteNodeContext, snapshot};

#[test]
fn worker_snapshot_stays_available_when_advisory_image_cache_cannot_be_observed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let blocked_parent = temp.path().join("not-a-directory");
    std::fs::write(&blocked_parent, "blocks image cache sqlite parent")
        .expect("write cache parent blocker");
    let context = RemoteNodeContext::isolated_for_test().with_image_cache_config(
        RemoteImageCacheRuntimeConfig {
            db_path: blocked_parent.join("agent.sqlite"),
            budget_bytes: 1,
            mutable_tag_ttl_secs: 1,
            sweep_interval_secs: 1,
            low_disk_min_free_percent: 1.0,
            low_disk_min_free_bytes: 1,
        },
    );

    assert!(
        context.node_status().is_ok(),
        "core node status stays valid"
    );
    let response = snapshot::handle(&context, "GET");

    assert_eq!(response.status_code, 200);
    let snapshot = tak_proto::worker_v2::decode_snapshot(&response.body)
        .expect("decode available worker snapshot");
    assert!(snapshot.cached_content.is_empty());
}
