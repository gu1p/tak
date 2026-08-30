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
    let requests = daemon.finish_expecting(6);

    for output in &outputs {
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let rendered = outputs
        .iter()
        .map(|output| String::from_utf8_lossy(&output.stdout))
        .collect::<Vec<_>>();
    assert!(rendered[0].contains("run-1") && rendered[0].contains("running"));
    assert!(rendered[1].contains("//:check") && rendered[1].contains("worker-a"));
    assert!(rendered[2].contains("succeeded") && rendered[2].contains("done"));
    assert!(rendered[3].contains("cancelling"));
    assert_eq!(
        fs::read(destination.join("result.txt")).unwrap(),
        b"artifact"
    );
    assert_eq!(requests.len(), 6);
}
