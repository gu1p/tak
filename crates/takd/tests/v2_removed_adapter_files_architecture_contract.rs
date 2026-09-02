use std::path::Path;

#[test]
fn daemon_has_no_v1_adapter_files_and_keeps_worker_v2_dispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for removed in [
        "src/daemon/protocol/daemon_tasks.rs",
        "src/daemon/protocol/dispatch.rs",
        "src/daemon/protocol/request_wire.rs",
        "src/daemon/protocol/request_wire/decode.rs",
        "src/daemon/protocol/request_wire/encode.rs",
        "src/daemon/protocol/request_wire/envelope.rs",
        "src/daemon/protocol/dispatch/remote.rs",
        "src/daemon/remote/route_submit.rs",
        "src/daemon/remote/submit_payload_parse.rs",
        "src/daemon/remote/worker_submit_execution.rs",
        "src/daemon/remote/active_executions.rs",
        "src/daemon/remote/types/context_active_executions.rs",
        "src/daemon/remote/status_job_metadata.rs",
        "src/daemon/remote/route_node.rs",
    ] {
        assert!(
            !root.join(removed).exists(),
            "legacy adapter remains: {removed}"
        );
    }

    let worker = read(root, "src/daemon/remote/route_worker_v2.rs");
    assert!(worker.contains("/v2/attempts/"));
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect(relative)
}
