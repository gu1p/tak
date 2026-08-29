use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn explicit_v2_stops_before_legacy_client_execution() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let env = BTreeMap::from([(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    )]);
    write_tasks(
        &workspace,
        r#"SPEC = module_spec(
  spec_version=2,
  tasks=[task("check", steps=[cmd("sh", "-c", "echo ran > should-not-run")])],
)
SPEC
"#,
    )
    .expect("write tasks");

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).expect("run tak");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "stderr:\n{stderr}");
    for token in [
        "spec_version=2",
        "does not load or execute v2 modules",
        "cannot fall back to legacy client execution",
    ] {
        assert!(stderr.contains(token), "missing `{token}`:\n{stderr}");
    }
    assert!(!stderr.contains("type errors"), "{stderr}");
    assert!(!workspace.join("should-not-run").exists());
}
