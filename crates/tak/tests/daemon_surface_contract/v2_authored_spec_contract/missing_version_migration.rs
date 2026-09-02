use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn omitted_spec_version_is_rejected_before_client_execution() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SPEC = module_spec(tasks=[
  task("check", steps=[cmd("sh", "-c", "echo ran > should-not-run")])
])
SPEC
"#,
    )
    .unwrap();
    let env = BTreeMap::from([(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    )]);

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "{stderr}");
    for token in [
        "spec_version=2",
        "Migration summary",
        "tak docs dump",
        "daemon-owned",
    ] {
        assert!(stderr.contains(token), "missing `{token}`:\n{stderr}");
    }
    assert!(!workspace.join("should-not-run").exists());
}
