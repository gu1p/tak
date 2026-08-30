use std::collections::{BTreeMap, HashMap};

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{run_tak_output, write_tasks};

#[test]
fn shared_workspace_tasks_observe_undeclared_writes_without_copying_them_back() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "../d.sock".into())]);

    let output = run_tak_output(&workspace, &["run", "//:consumer"], &environment).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("shared-workspace-visible"), "{stdout}");
    assert!(!workspace.join(".shared").exists());
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-shared-workspace".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"BUILD = session(
  "build",
  execution=Execution.Local(),
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=1),
  affinity=Affinity.RequireSameNode("build"),
)
SPEC = module_spec(
  spec_version=2,
  tasks=[
    task("producer", steps=[cmd("sh", "-c",
      "mkdir -p .shared && printf producer > .shared/value")], use_session=BUILD),
    task("consumer", deps=[":producer"], steps=[cmd("sh", "-c",
      "test \"$(cat .shared/value)\" = producer && printf 'shared-workspace-visible\\n'")],
      use_session=BUILD),
  ],
)
SPEC
"#;
