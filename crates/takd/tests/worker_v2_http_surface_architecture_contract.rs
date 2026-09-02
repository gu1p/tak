use std::path::Path;

#[test]
fn worker_http_surface_has_no_v1_adapter_modules_or_symbols() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for removed in [
        "src/daemon/remote/http2_roundtrip_support.rs",
        "src/daemon/remote/http2_roundtrip_support/upload.rs",
        "src/daemon/remote/http2_roundtrip_tests.rs",
        "src/daemon/remote/route_node.rs",
        "src/daemon/remote/route_logs.rs",
        "src/daemon/remote/route_tasks.rs",
        "src/daemon/remote/route_events.rs",
        "src/daemon/remote/route_result.rs",
        "src/daemon/remote/route_outputs.rs",
        "src/daemon/remote/route_uploads.rs",
        "src/daemon/remote/route_uploads/storage.rs",
        "src/daemon/remote/route_uploads/storage/stream.rs",
        "src/daemon/remote/route_uploads/stream.rs",
        "src/daemon/remote/route_uploads/wormhole.rs",
        "tests/support/remote_v1_http.rs",
        "tests/support/remote_v1_http_submit.rs",
    ] {
        assert!(
            !root.join(removed).exists(),
            "legacy worker route remains: {removed}"
        );
    }

    for source in [
        "src/daemon/remote/mod.rs",
        "src/daemon/remote/router.rs",
        "src/daemon/remote/http_server.rs",
        "src/daemon/remote/http_server/http2.rs",
        "src/daemon/remote/http_server/http2/request_body.rs",
        "src/daemon/remote/http_server/response.rs",
        "src/daemon/remote/query_helpers.rs",
        "src/daemon/remote/route_worker_v2.rs",
        "src/daemon/remote/route_worker_v2/attempts.rs",
        "src/daemon/remote/route_worker_v2/identity.rs",
        "src/daemon/remote/route_worker_v2/observations.rs",
        "src/daemon/remote/route_worker_v2/snapshot.rs",
        "src/daemon/remote/route_worker_v2/workspace_cache.rs",
        "src/daemon/remote/types/records.rs",
        "src/service.rs",
        "src/service/control.rs",
        "src/service/tor/rend.rs",
    ] {
        let body = std::fs::read_to_string(root.join(source)).expect(source);
        assert!(
            !body.contains("remote_v1"),
            "legacy symbol remains in {source}"
        );
        assert!(
            !body.contains("RemoteV1"),
            "legacy type remains in {source}"
        );
    }

    let router = std::fs::read_to_string(root.join("src/daemon/remote/router.rs")).unwrap();
    assert!(router.contains("path.starts_with(\"/v1/\")"));
    assert!(router.contains("PROTOCOL_V2_UPGRADE_MESSAGE"));

    let helpers = std::fs::read_to_string(root.join("src/daemon/remote/query_helpers.rs")).unwrap();
    assert!(!helpers.contains("remote_task_path_arg"));

    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("magic-wormhole"));
}

#[test]
fn daemon_owned_health_callers_use_worker_v2_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for source in [
        "src/daemon/peer_manager/heartbeat.rs",
        "src/service/tor/probe/http_client.rs",
        "src/cli/tasks_output/client.rs",
    ] {
        let body = std::fs::read_to_string(root.join(source)).expect(source);
        assert!(
            !body.contains("/v1/"),
            "legacy health request remains in {source}"
        );
        assert!(
            body.contains("/v2/worker/"),
            "missing worker-v2 route in {source}"
        );
    }
}
