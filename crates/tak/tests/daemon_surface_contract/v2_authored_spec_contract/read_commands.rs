use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn existing_read_commands_render_explicit_v2_tasks_without_a_daemon() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SPEC = module_spec(spec_version=2, tasks=[
  task("dep", doc="prepare input", steps=[cmd("true")]),
  task("check", doc="verify everything", deps=[":dep"], timeout_s=7,
       steps=[cmd("true")]),
])
SPEC
"#,
    )
    .unwrap();
    let environment = BTreeMap::new();

    let cases = [
        (
            vec!["list"],
            vec!["//:dep", "//:check", "verify everything"],
        ),
        (vec!["tree"], vec!["//:check", "//:dep"]),
        (
            vec!["explain", "//:check"],
            vec!["label: //:check", "//:dep", "timeout_s: 7"],
        ),
        (
            vec!["graph", "//:check"],
            vec!["digraph", "//:dep", "//:check"],
        ),
    ];
    for (argv, expected) in cases {
        let output = run_tak_output(&workspace, &argv, &environment).unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{argv:?}: {stderr}");
        for token in expected {
            assert!(
                stdout.contains(token),
                "{argv:?} missing `{token}`:\n{stdout}"
            );
        }
    }
}
