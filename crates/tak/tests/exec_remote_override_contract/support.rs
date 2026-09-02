use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;

use crate::daemon_surface_contract::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

pub(super) struct Captured {
    pub(super) output: Output,
    requests: Vec<Value>,
}

impl Captured {
    pub(super) fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    pub(super) fn runtime(&self) -> &Value {
        &self.requests[1]["operation"]["run"]["tasks"][0]["runtime"]
    }

    pub(super) fn candidate(&self) -> &Value {
        &self.requests[1]["operation"]["run"]["jobs"][0]["placement_candidates"][0]
    }
}

pub(super) fn run(args: &[&str]) -> Captured {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("docker")).unwrap();
    fs::write(workspace.join("docker/Dockerfile"), "FROM alpine:3.20\n").unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RemoteSubmissionFlow);
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);
    let output = run_tak_output(&workspace, args, &environment).unwrap();
    let requests = daemon.finish_expecting(6);
    assert_eq!(requests[0]["operation"]["type"], "ResolveRemoteCandidates");
    assert_eq!(requests[1]["operation"]["type"], "SubmitRun");
    Captured { output, requests }
}
