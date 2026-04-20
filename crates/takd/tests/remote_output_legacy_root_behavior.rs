use std::fs;

use rusqlite::{Connection, params};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_output_route_uses_context_execution_root_for_legacy_submit_rows_without_persisted_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path.clone()).expect("store");
    let exec_root_base = temp.path().join("legacy-exec-root");
    let context = RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests().with_explicit_remote_exec_root(exec_root_base.clone()),
    );

    let conn = Connection::open(&db_path).expect("open sqlite");
    conn.execute(
        "INSERT INTO submit_attempts (
            idempotency_key, task_run_id, attempt, selected_node_id, execution_root_base, created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["run-legacy:1", "run-legacy", 1, "builder-a", "", 0_i64],
    )
    .expect("insert legacy submit");

    let artifact_root = temp
        .path()
        .join("takd-remote-artifacts")
        .join("run-legacy_1");
    let nested = artifact_root.join("nested");
    fs::create_dir_all(&nested).expect("artifact dirs");
    fs::write(nested.join("output.txt"), b"legacy output").expect("artifact file");

    let response = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/run-legacy/outputs?path=nested%2Foutput.txt",
        None,
    )
    .expect("route response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.body, b"legacy output");
}
