use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

const RESUME_OFFSET: usize = 257;
const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("check", steps=[cmd("true")])])
SPEC
"#;

#[test]
fn present_workspace_skips_upload_then_commits_and_attaches() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::WorkspacePresentSubmissionFlow);

    let output = run_tak_output(&workspace, &["run", "//:check"], &environment(&socket)).unwrap();
    let requests = daemon.finish_expecting(4);

    assert!(output.status.success());
    assert_eq!(
        operations(&requests),
        ["SubmitRun", "CommitRun", "AttachRun", "GetOutputManifest"]
    );
}

#[test]
fn resumable_upload_sends_only_archive_tail_at_daemon_offset() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    std::fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let baseline_socket = temp.path().join("baseline.sock");
    let baseline = FakeRunDaemon::spawn(&baseline_socket, Reply::SubmissionFlow);
    let output = run_tak_output(
        &workspace,
        &["run", "//:check"],
        &environment(&baseline_socket),
    )
    .unwrap();
    assert!(output.status.success());
    let baseline_requests = baseline.finish_expecting(5);
    let full_archive = upload_chunk(&baseline_requests[1]);
    assert!(full_archive.len() > RESUME_OFFSET);

    let resumed_socket = temp.path().join("resumed.sock");
    let resumed = FakeRunDaemon::spawn(
        &resumed_socket,
        Reply::ResumableSubmissionFlow(RESUME_OFFSET as u64),
    );
    let output = run_tak_output(
        &workspace,
        &["run", "//:check"],
        &environment(&resumed_socket),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = resumed.finish_expecting(5);
    let upload = &requests[1]["operation"];
    assert_eq!(upload["offset"], RESUME_OFFSET as u64);
    assert_eq!(upload["archive_size"], full_archive.len() as u64);
    assert_eq!(upload_chunk(&requests[1]), full_archive[RESUME_OFFSET..]);
    assert_eq!(
        operations(&requests),
        [
            "SubmitRun",
            "UploadWorkspace",
            "CommitRun",
            "AttachRun",
            "GetOutputManifest"
        ]
    );
}

fn environment(socket: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())])
}

fn operations(requests: &[serde_json::Value]) -> Vec<&str> {
    requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect()
}

fn upload_chunk(request: &serde_json::Value) -> Vec<u8> {
    STANDARD
        .decode(request["operation"]["chunk_base64"].as_str().unwrap())
        .unwrap()
}
