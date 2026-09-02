use std::fs;

use tak_core::v2::{Affinity, RemoteSelection, SessionReuse};
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn explicit_v2_root_is_inspected_as_v2_domain_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SHARED = session(
  "build",
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
  affinity=Affinity.RequireSameNode("build"),
)
SPEC = module_spec(
  spec_version=2,
  project_id="sample",
  defaults=Defaults(pass_env=["ROOT_TOKEN"]),
  tasks=[task(
    "check",
    steps=[cmd("echo", "ok")],
    outputs=[path("result.txt")],
    execution=Execution.Remote(
      transport=Transport.TorOnionService(),
      session=SHARED,
    ),
    idempotent=True,
    pass_env=["TASK_TOKEN"],
  )],
)
SPEC
"#,
    )
    .expect("write tasks");

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .expect("inspect v2 root");
    assert_eq!(root.module.project_id.as_deref(), Some("sample"));
    assert_eq!(root.module.defaults.pass_env.as_strs(), ["ROOT_TOKEN"]);
    let task = &root.module.tasks[0];
    assert!(task.idempotent);
    assert_eq!(task.pass_env.as_strs(), ["TASK_TOKEN"]);
    let remote = task
        .execution
        .as_ref()
        .expect("execution")
        .remote()
        .expect("remote");
    assert_eq!(remote.selection, RemoteSelection::Balanced);
    assert_eq!(remote.transport.as_deref(), Some("tor"));
    let session = remote.session.as_ref().expect("session");
    assert_eq!(
        session.reuse,
        SessionReuse::shared_workspace(2).expect("reuse")
    );
    assert_eq!(
        session.affinity,
        Some(Affinity::require_same_node("build").expect("affinity"))
    );
}

#[test]
fn shared_workspace_task_cannot_weaken_the_loaded_session_affinity() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SHARED = session(
  "build",
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
  affinity=Affinity.RequireSameNode("build"),
)
SPEC = module_spec(spec_version=2, tasks=[task(
  "check",
  execution=Execution.Remote(session=SHARED),
  affinity=Affinity.PreferSameNode("build"),
)])
SPEC
"#,
    )
    .expect("write tasks");

    let error = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .expect_err("soft task affinity must not weaken a shared workspace")
        .to_string();
    assert!(error.contains("cannot weaken or change"), "{error}");
}
