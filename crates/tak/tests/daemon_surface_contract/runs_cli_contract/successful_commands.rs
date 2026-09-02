use std::collections::BTreeMap;
use std::fs;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn run_management_commands_render_persisted_state_and_retrieve_outputs() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::ManagementFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let destination = root.path().join("outputs");
    let destination_arg = destination.display().to_string();
    let cases = [
        vec!["runs", "list"],
        vec!["runs", "show", "run-1"],
        vec!["runs", "attach", "run-1"],
        vec!["runs", "cancel", "run-1"],
        vec!["runs", "outputs", "run-1", "--to", &destination_arg],
    ];
    let outputs = cases
        .iter()
        .map(|arguments| run_tak_output(root.path(), arguments, &environment).expect("run command"))
        .collect::<Vec<_>>();
    let requests = daemon.finish_expecting(7);

    for (index, output) in outputs.iter().enumerate() {
        if index == 2 {
            assert!(!output.status.success());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("original checkout association for run run-1 is unavailable")
                    && stderr.contains("tak runs outputs run-1 --to DIR"),
                "{stderr}"
            );
        } else {
            assert!(
                output.status.success(),
                "stdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    let rendered = outputs
        .iter()
        .map(|output| String::from_utf8_lossy(&output.stdout))
        .collect::<Vec<_>>();
    assert!(rendered[0].contains("run-1") && rendered[0].contains("running"));
    assert!(rendered[1].contains("//:check") && rendered[1].contains("worker-a"));
    assert!(rendered[1].contains("cache=miss"), "{}", rendered[1]);
    assert!(rendered[1].contains("//:cached") && rendered[1].contains("worker-b"));
    assert!(rendered[1].contains("cache=hit"), "{}", rendered[1]);
    assert!(rendered[2].contains("succeeded") && rendered[2].contains("done"));
    assert!(rendered[3].contains("cancelling"));
    assert_eq!(
        fs::read(destination.join("result.txt")).unwrap(),
        b"artifact"
    );
    let operation_types = requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        operation_types,
        [
            "ListRuns",
            "GetRun",
            "AttachRun",
            "GetOutputManifest",
            "CancelRun",
            "GetOutputManifest",
            "GetOutputChunk",
        ]
    );
}
