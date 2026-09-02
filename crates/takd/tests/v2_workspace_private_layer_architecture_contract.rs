use std::path::PathBuf;

#[test]
fn local_and_worker_private_workspaces_share_the_reflink_copy_layer() {
    let daemon = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/daemon");
    let local =
        std::fs::read_to_string(daemon.join("local_attempt_transport/workspace.rs")).unwrap();
    let worker_root = daemon.join("remote/worker_v2_execution");
    let worker = std::fs::read_to_string(worker_root.join("workspace.rs")).unwrap()
        + &std::fs::read_to_string(worker_root.join("workspace/private.rs")).unwrap();
    let layer = std::fs::read_to_string(daemon.join("workspace_layer.rs")).unwrap();
    let path_cache = std::fs::read_to_string(daemon.join("path_cache/tree.rs")).unwrap();
    let buffered =
        std::fs::read_to_string(daemon.join("workspace_layer/buffered_copy.rs")).unwrap();

    assert!(local.contains("workspace_layer::private_copy"), "{local}");
    assert!(worker.contains("workspace_layer::private_copy"), "{worker}");
    assert!(
        path_cache.contains("workspace_layer::private_copy_shallow"),
        "{path_cache}"
    );
    assert!(!path_cache.contains("fs::copy"), "{path_cache}");
    assert!(layer.contains("try_reflink"), "{layer}");
    assert!(layer.contains("buffered_copy"), "{layer}");
    assert!(buffered.contains("source.read(&mut buffer)"), "{buffered}");
    assert!(buffered.contains("destination.write_all"), "{buffered}");
    assert!(!buffered.contains("fs::copy"), "{buffered}");
}
