use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;
use takd::RunStore;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{tak_bin, write_tasks};

#[test]
fn independent_dependency_outputs_fail_once_before_the_consumer_runs() {
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
        .args(["run", "//:consume"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("XDG_STATE_HOME", "../state")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let store = RunStore::with_db_path(socket.with_extension("v2.sqlite")).unwrap();
    wait_for_terminal(&store, &mut child);
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains(
            "independent producers conflict on declared output `shared/value.txt` before `//:consume`"
        ),
        "{stdout}"
    );
    assert!(stderr.contains("did not succeed"), "{stderr}");
}

fn wait_for_terminal(store: &RunStore, child: &mut std::process::Child) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if store
            .list_runs()
            .unwrap()
            .first()
            .is_some_and(|run| run.state.is_terminal())
        {
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!(
                "output conflict did not terminate: {:?}",
                store.list_runs().unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-dependency-output-conflict".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[
  task("left", outputs=[path("shared/value.txt")], steps=[cmd("sh", "-c", "mkdir -p shared; printf left > shared/value.txt")]),
  task("right", outputs=[path("shared/value.txt")], steps=[cmd("sh", "-c", "mkdir -p shared; printf right > shared/value.txt")]),
  task("consume", deps=[":left", ":right"], steps=[cmd("sh", "-c", "exit 99")]),
])
SPEC
"#;
