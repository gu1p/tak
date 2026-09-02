use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn docker_run_gets_concrete_candidates_from_takd_and_submits_balanced() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RemoteSubmissionFlow);
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "--node",
            "worker-a",
            "--arch",
            "arm64",
            "--os",
            "linux",
            "--pool",
            "build",
            "--tag",
            "builder",
            "--capability",
            "docker",
            "--transport",
            "direct",
            "docker",
            "run",
            "alpine:3.20",
            "true",
        ],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(6);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stderr}");
    assert_eq!(requests[0]["operation"]["type"], "ResolveRemoteCandidates");
    let requirements = &requests[0]["operation"]["requirements"];
    assert_eq!(requirements["pool"], "build");
    assert_eq!(requirements["required_tags"], json!(["builder"]));
    assert_eq!(requirements["transport"], "direct");
    assert_eq!(
        requirements["required_capabilities"],
        json!(["arch:arm64", "docker", "node:worker-a", "os:linux"])
    );
    let run = &requests[1]["operation"]["run"];
    assert_eq!(run["jobs"][0]["placement_policy"]["selection"], "balanced");
    assert_eq!(
        run["jobs"][0]["placement_candidates"][0]["node_id"],
        "worker-a"
    );
    assert_eq!(run["tasks"][0]["runtime"]["source"]["image"], "alpine:3.20");
}
