use std::fs;

use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn mixed_remote_selections_have_an_actionable_diagnostic() {
    let error = load_error(
        r#"Execution.FirstAvailable([
  Execution.Remote(selection=RemoteSelection.Balanced()),
  Execution.Remote(selection=RemoteSelection.RoundRobin()),
])"#,
    );
    assert!(error.contains("same RemoteSelection"), "{error}");
}

#[test]
fn mixed_placement_runtimes_have_an_actionable_diagnostic() {
    let error = load_error(
        r#"Execution.FirstAvailable([
  Execution.Remote(container=Container.Image("rust:latest")),
  Execution.Local(),
])"#,
    );
    assert!(error.contains("same container runtime"), "{error}");
}

fn load_error(execution: &str) -> String {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let source = format!(
        "SPEC=module_spec(spec_version=2, tasks=[task(\"check\", execution={execution})])\nSPEC\n"
    );
    fs::write(temp.path().join("TASKS.py"), source).unwrap();
    inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .unwrap_err()
        .to_string()
}
