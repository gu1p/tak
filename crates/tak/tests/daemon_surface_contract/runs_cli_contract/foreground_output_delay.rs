use std::collections::BTreeMap;
use std::time::Duration;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn foreground_run_waits_for_final_output_manifest_after_terminal_success() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedOutputSubmissionFlow(Duration::from_millis(2_500)),
    );
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "../d.sock".into())]);

    let output = run_tak_output(&workspace, &["run", "//:check"], &environment).unwrap();
    let requests = daemon.finish_expecting(6);

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(workspace.join("generated.txt")).unwrap(),
        b"artifact"
    );
    assert_eq!(
        requests
            .iter()
            .map(|request| request["operation"]["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "SubmitRun",
            "UploadWorkspace",
            "CommitRun",
            "AttachRun",
            "GetOutputManifest",
            "GetOutputChunk",
        ]
    );
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("check", outputs=[path("generated.txt")], steps=[cmd("true")])])
SPEC
"#;
