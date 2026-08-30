use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn remote_v2_stops_before_submission_until_daemon_candidate_resolution_is_available() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let env = BTreeMap::from([
        (
            "XDG_STATE_HOME".into(),
            temp.path().join("state").display().to_string(),
        ),
        ("BUILD_TOKEN".into(), "do-not-render-build".into()),
        ("TASK_TOKEN".into(), "do-not-render-task".into()),
    ]);
    write_tasks(
        &workspace,
        r#"SHARED = session(
  "build",
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
  affinity=Affinity.RequireSameNode("build"),
)
SPEC = module_spec(
  spec_version=2,
  defaults=Defaults(pass_env=["BUILD_TOKEN"]),
  tasks=[task(
    "check",
    steps=[cmd("sh", "-c", "echo ran > should-not-run")],
    outputs=[path("result.txt")],
    execution=Execution.Remote(
      selection=RemoteSelection.Balanced(),
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

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).expect("run tak");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "stderr:\n{stderr}");
    assert!(
        stderr.contains("takd remote placement candidate resolution"),
        "stderr:\n{stderr}"
    );
    for secret in ["do-not-render-build", "do-not-render-task"] {
        assert!(!stderr.contains(secret), "secret rendered: {stderr}");
    }
    for stale in [
        "does not load or execute v2 modules",
        "type errors",
        "takd serve",
        "loaded and validated",
    ] {
        assert!(!stderr.contains(stale), "unexpected `{stale}`: {stderr}");
    }
    assert!(!workspace.join("should-not-run").exists());
}
