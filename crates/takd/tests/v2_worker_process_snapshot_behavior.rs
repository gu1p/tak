use std::process::Command;

use tak_proto::NodeInfo;
use tak_proto::worker_v2::decode_snapshot;
use takd::{RemoteNodeContext, SubmitAttemptStore};

#[test]
fn worker_snapshot_reports_name_and_argv_without_process_environment() {
    std::fs::create_dir_all(".tmp").unwrap();
    let marker = format!("tak-worker-process-{}", uuid::Uuid::new_v4());
    let secret = format!("tak-worker-secret-{}", uuid::Uuid::new_v4());
    let mut process = Command::new("/bin/sh")
        .args(["-c", "trap ':' EXIT; sleep 30", &marker])
        .env("TAK_PASSED_SECRET", &secret)
        .spawn()
        .unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "worker-a".into(),
            display_name: "worker-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec![],
            tags: vec![],
            capabilities: vec![],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        crate::support::runtime_config::builder()
            .with_skip_exec_root_probe(true)
            .build(),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let response = takd::daemon::remote::handle_worker_http_request(
        &context,
        &store,
        "GET",
        "/v2/worker/snapshot",
        &[("x-tak-protocol-version".into(), "v2".into())],
        None,
    )
    .unwrap();
    process.kill().unwrap();
    process.wait().unwrap();

    let snapshot = decode_snapshot(&response.body).unwrap();
    assert!(snapshot.processes.iter().any(|observed| {
        observed.name.contains(&marker)
            || observed
                .arguments
                .iter()
                .any(|argument| argument.contains(&marker))
    }));
    assert!(!String::from_utf8_lossy(&response.body).contains(&secret));
}
