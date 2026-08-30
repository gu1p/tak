use std::collections::BTreeMap;

use crate::support::run_tak_output;

#[test]
fn runs_help_is_reachable_and_visible_without_a_workspace() {
    let root = tempfile::tempdir().expect("temp root");
    let env = BTreeMap::new();
    let root_help = run_tak_output(root.path(), &["--help"], &env).expect("root help");
    assert!(root_help.status.success(), "root help should succeed");
    let root_stdout = String::from_utf8_lossy(&root_help.stdout);
    assert!(
        root_stdout
            .lines()
            .any(|line| line.trim_start().starts_with("runs ")),
        "runs command should be visible\n{root_stdout}"
    );
    let docs = run_tak_output(root.path(), &["docs", "dump"], &env).expect("docs dump");
    assert!(docs.status.success(), "docs dump should succeed");
    let docs_stdout = String::from_utf8_lossy(&docs.stdout);
    assert!(docs_stdout.contains("### `tak runs"), "{docs_stdout}");

    let help = run_tak_output(root.path(), &["runs", "--help"], &env).expect("runs help");
    assert!(help.status.success(), "runs help should succeed");
    let stdout = String::from_utf8_lossy(&help.stdout);
    for command in ["list", "show", "attach", "cancel", "outputs"] {
        assert!(
            stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{command} "))),
            "missing `{command}` command\n{stdout}"
        );
    }

    for command in ["list", "show", "attach", "cancel", "outputs"] {
        let leaf = run_tak_output(root.path(), &["runs", command, "--help"], &env)
            .expect("runs leaf help");
        assert!(leaf.status.success(), "{command} help should succeed");
        let leaf_stdout = String::from_utf8_lossy(&leaf.stdout);
        if command != "list" {
            assert!(leaf_stdout.contains("<RUN_ID>"), "{command}: {leaf_stdout}");
        }
        if command == "outputs" {
            assert!(leaf_stdout.contains("--to <DIR>"), "{leaf_stdout}");
        }
    }
}
