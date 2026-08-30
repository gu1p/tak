use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn explicit_v2_is_validated_then_stops_before_submission_without_fallback() {
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
    for token in [
        "module_spec(spec_version=2) loaded and validated",
        "daemon-owned v2 graph resolution/submission is not active",
        "no legacy WorkspaceSpec was produced",
        "no client executor fallback was attempted",
    ] {
        assert!(stderr.contains(token), "missing `{token}`:\n{stderr}");
    }
    for secret in ["do-not-render-build", "do-not-render-task"] {
        assert!(!stderr.contains(secret), "secret rendered: {stderr}");
    }
    for stale in [
        "does not load or execute v2 modules",
        "type errors",
        "takd serve",
    ] {
        assert!(!stderr.contains(stale), "unexpected `{stale}`: {stderr}");
    }
    assert!(!workspace.join("should-not-run").exists());
}
