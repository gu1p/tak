use std::fs;

use super::repo_root;

#[test]
fn retention_docs_distinguish_payload_ttl_from_blob_lru() {
    let guide = fs::read_to_string(repo_root().join("docs/daemon-runs-v2.md")).expect("v2 guide");
    let architecture = fs::read_to_string(repo_root().join("crates/takd/ARCHITECTURE.md"))
        .expect("takd architecture");

    assert!(
        guide.contains("terminal logs and outputs are retained for 7 days"),
        "v2 guide must document the payload retention TTL"
    );
    assert!(
        guide.contains("Workspace/path blobs instead use a configurable 20 GiB LRU budget"),
        "v2 guide must document budget-based blob retention"
    );
    assert!(
        architecture.contains("applies a configurable 20 GiB LRU budget to workspace/path blobs"),
        "takd architecture must document budget-based blob retention"
    );
    for (name, body) in [("v2 guide", guide), ("takd architecture", architecture)] {
        assert!(
            !body.contains("logs, outputs, and workspace/path blobs"),
            "{name} must not claim that cache blobs use the payload TTL"
        );
    }
}
