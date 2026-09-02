use std::fs;

use tak_loader::{LoadOptions, load_workspace};

fn load_error(source: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("workspace should fail")
        .to_string()
}

#[test]
fn omitted_version_is_rejected_with_the_v2_migration_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "Affinity = 'legacy metadata'\nSPEC = module_spec(tasks=[task('check', steps=[cmd('true')])])\nSPEC\n",
    )
    .expect("write tasks");

    let message = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("omitted version must fail")
        .to_string();
    assert!(
        message.contains("explicit spec_version=2 is required"),
        "{message}"
    );
    assert!(message.contains("Migration summary"), "{message}");
    assert!(message.contains("`max_pending`"), "{message}");
    assert!(message.contains("use queue `slots`"), "{message}");
    assert!(message.contains("Container `command`"), "{message}");
    assert!(message.contains("use task steps"), "{message}");
}

#[test]
fn positional_v2_marker_has_keyword_migration_guidance() {
    let message = load_error("SPEC = module_spec([], 2)\nSPEC\n");

    assert!(
        message.contains("declare spec_version=2 as a keyword argument"),
        "{message}"
    );
}

#[test]
fn explicit_v1_migration_precedes_legacy_boundary_validation() {
    let message =
        load_error("BROKEN = Runtime.Host()\nSPEC = module_spec(tasks=[], spec_version=1)\nSPEC\n");

    assert!(message.contains("TASKS.py:2:"), "{message}");
    assert!(message.contains("spec_version=1"), "{message}");
    assert!(message.contains("Migration summary"), "{message}");
    assert!(message.contains("spec_version=2"), "{message}");
    assert!(!message.contains("`Runtime` was replaced"), "{message}");
}

#[test]
fn explicit_v2_produces_the_read_only_workspace_view() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[], spec_version=2)\nSPEC\n",
    )
    .expect("write tasks");

    let workspace = load_workspace(temp.path(), &LoadOptions::default()).expect("v2 view");
    assert!(workspace.tasks.is_empty());
}

#[test]
fn unsupported_and_dynamic_versions_have_authored_diagnostics() {
    for (argument, expected) in [
        ("3", "protocol v2 is required"),
        ("VERSION", "spec_version must be the integer literal 2"),
    ] {
        let source =
            format!("VERSION = 2\nSPEC = module_spec(tasks=[], spec_version={argument})\nSPEC\n");
        let message = load_error(&source);
        assert!(message.contains("TASKS.py:2:"), "{message}");
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn syntax_errors_precede_version_classification() {
    let message = load_error("SPEC = module_spec(tasks=[], spec_version=1\n");

    assert!(message.contains("failed to parse"), "{message}");
    assert!(!message.contains("Migration summary"), "{message}");
}

#[test]
fn explicit_v2_uses_the_v2_evaluator() {
    let message =
        load_error("POISON = 1 / 0\nSPEC = module_spec(tasks=[], spec_version=2)\nSPEC\n");

    assert!(message.contains("failed to evaluate"), "{message}");
    assert!(message.contains("division by zero"), "{message}");
}
