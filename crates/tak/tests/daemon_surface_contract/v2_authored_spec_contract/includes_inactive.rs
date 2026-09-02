use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn v2_includes_are_loaded_for_existing_read_commands() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        "SPEC=module_spec(spec_version=2, tasks=[], includes=[path('apps/web')])\nSPEC\n",
    )
    .unwrap();
    write_tasks(
        &workspace.join("apps/web"),
        r#"SPEC=module_spec(spec_version=2, tasks=[
  task("check", doc="Included web check", steps=[cmd("true")]),
])
SPEC
"#,
    )
    .unwrap();

    let output = run_tak_output(&workspace, &["list"], &BTreeMap::new()).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("//apps/web:check"), "{stdout}");
    assert!(stdout.contains("Included web check"), "{stdout}");
}
