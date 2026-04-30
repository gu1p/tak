use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn rejects_host_runtime_outside_local_execution() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[
  task("bad", steps=[cmd("true")], execution=Execution.Remote(container=Container.Host())),
])
SPEC
"#,
    )
    .expect("write TASKS.py");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("host remote");
    assert!(
        err.to_string()
            .contains("`Container.Host(...)` is unsupported."),
        "{err:#}"
    );
}
