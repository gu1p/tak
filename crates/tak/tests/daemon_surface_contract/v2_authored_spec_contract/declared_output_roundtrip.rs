use std::collections::{BTreeMap, HashMap};

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{run_tak_output, write_tasks};

#[test]
fn foreground_overlays_declared_outputs_and_materializes_only_declared_results() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(&workspace, &["run", "//:consume"], &environment).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("output_committing"), "{stdout}");
    assert_eq!(
        std::fs::read(workspace.join("generated/input.txt")).unwrap(),
        b"producer"
    );
    assert_eq!(
        std::fs::read(workspace.join("dist/result.txt")).unwrap(),
        b"producer-consumed"
    );
    assert!(!workspace.join("scratch/leak.txt").exists());
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-declared-output".into(),
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
  reuse=SessionReuse.Workspace(),
)
SPEC = module_spec(
  spec_version=2,
  tasks=[
    task("produce", outputs=[path("generated/input.txt")], use_session=BUILD,
      steps=[cmd("sh", "-c", "mkdir -p generated scratch && printf producer > generated/input.txt && printf leak > scratch/leak.txt")]),
    task("consume", deps=[":produce"], outputs=[path("dist/result.txt")], use_session=BUILD,
      steps=[cmd("sh", "-c", "test \"$(cat generated/input.txt)\" = producer && test ! -e scratch/leak.txt && mkdir -p dist && printf producer-consumed > dist/result.txt")]),
  ],
)
SPEC
"#;
