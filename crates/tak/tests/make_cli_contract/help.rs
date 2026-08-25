use crate::support::run_tak_output;

use std::collections::BTreeMap;

use anyhow::Result;

#[test]
fn make_help_exposes_the_goal_and_execution_controls() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let env = BTreeMap::new();
    let root_output = run_tak_output(workspace.path(), &["--help"], &env)?;
    assert!(
        root_output.status.success(),
        "status: {:?}",
        root_output.status
    );
    let root_help = String::from_utf8_lossy(&root_output.stdout);
    assert!(
        root_help
            .lines()
            .any(|line| line.trim_start().starts_with("make ")),
        "root help should expose `make`:\n{root_help}"
    );
    let make_output = run_tak_output(workspace.path(), &["make", "--help"], &env)?;
    assert!(
        make_output.status.success(),
        "status: {:?}\nstderr:\n{}",
        make_output.status,
        String::from_utf8_lossy(&make_output.stderr)
    );
    let make_help = String::from_utf8_lossy(&make_output.stdout);
    assert!(make_help.contains("<GOAL>"), "make help:\n{make_help}");
    let exposes = |option| make_help.split_whitespace().any(|word| word == option);
    for option in [
        "--local",
        "--local-no-container",
        "--remote",
        "--container",
        "--container-image",
        "--container-dockerfile",
        "--container-build-context",
        "--parallel-output",
    ] {
        assert!(
            exposes(option),
            "make help missing `{option}`:\n{make_help}"
        );
    }
    Ok(())
}
