use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{tak_bin, write_tasks};

#[test]
fn foreground_v2_work_is_executed_and_streamed_by_the_real_daemon() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:target", "--pass-env", "TOKEN"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("TOKEN", "v2-secret")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let completed = loop {
        if child.try_wait().unwrap().is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            break false;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(completed, "tak did not finish\n{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
    for visible in [
        "transferring",
        "running",
        "daemon-local-marker",
        "succeeded",
    ] {
        assert!(stdout.contains(visible), "missing {visible}: {stdout}");
    }
    assert!(!workspace.join("client-executor-ran").exists());
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-local".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(
  spec_version=2,
  tasks=[task("target", steps=[cmd("/bin/sh", "-c",
    "test \"$TOKEN\" = v2-secret && printf 'daemon-local-marker\\n' && touch client-executor-ran")])],
)
SPEC
"#;
