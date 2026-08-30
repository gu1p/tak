use std::collections::BTreeMap;
use std::fs;

use serde_json::{Value, json};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn every_runs_command_sends_one_concrete_protocol_v2_operation() {
    let root = tempfile::tempdir().expect("temp root");
    write_tasks(
        root.path(),
        "raise RuntimeError('RUNS_MUST_NOT_LOAD_TASKS')\n",
    )
    .expect("write poison tasks");
    let destination = root.path().join("outputs");
    fs::create_dir(&destination).expect("create output destination");
    fs::write(destination.join("keep.txt"), "keep").expect("seed output destination");
    let destination_arg = destination.display().to_string();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::Inactive("UNTRUSTED_DAEMON_MESSAGE_MUST_NOT_RENDER"),
    );
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);
    let cases = [
        (vec!["runs", "list"], "tak-runs-list"),
        (vec!["runs", "show", "run-1"], "tak-runs-show"),
        (vec!["runs", "attach", "run-1"], "tak-runs-attach"),
        (vec!["runs", "cancel", "run-1"], "tak-runs-cancel"),
        (
            vec!["runs", "outputs", "run-1", "--to", &destination_arg],
            "tak-runs-outputs",
        ),
    ];
    let mut outputs = Vec::new();
    for (args, _) in &cases {
        outputs.push(run_tak_output(root.path(), args, &env).expect("run v2 command"));
    }
    let requests = daemon.finish_expecting(cases.len());
    let expected = [
        json!({"type": "ListRuns"}),
        json!({"type": "GetRun", "run_id": "run-1"}),
        json!({"type": "AttachRun", "run_id": "run-1", "after_event": 0}),
        json!({"type": "CancelRun", "run_id": "run-1"}),
        json!({"type": "GetOutputManifest", "run_id": "run-1"}),
    ];

    assert_eq!(
        requests.len(),
        expected.len(),
        "captured requests: {requests:?}"
    );
    for (((request, operation), (_, request_id)), output) in
        requests.iter().zip(expected).zip(cases).zip(outputs)
    {
        assert_eq!(request["protocol_version"], 2);
        assert_eq!(request["request_id"], request_id);
        assert_eq!(request["operation"], operation);
        assert_eq!(request.as_object().unwrap().len(), 3);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Protocol v2 run operations are not active"),
            "{stderr}"
        );
        assert!(!stderr.contains("UNTRUSTED_DAEMON_MESSAGE_MUST_NOT_RENDER"));
    }
    assert_eq!(
        fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
    assert!(
        !requests
            .iter()
            .any(|request| contains_value(request, &destination_arg))
    );
}

fn contains_value(value: &Value, needle: &str) -> bool {
    value.to_string().contains(needle)
}
