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
fn omitted_version_remains_a_temporary_legacy_bootstrap() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[task('check', steps=[cmd('true')])])\nSPEC\n",
    )
    .expect("write tasks");

    let workspace = load_workspace(temp.path(), &LoadOptions::default()).expect("legacy load");
    assert_eq!(workspace.tasks.len(), 1);
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
fn explicit_v2_is_recognized_before_the_legacy_type_stub() {
    let message = load_error("SPEC = module_spec(tasks=[], spec_version=2)\nSPEC\n");

    assert!(message.contains("TASKS.py:1:"), "{message}");
    assert!(message.contains("spec_version=2"), "{message}");
    assert!(
        message.contains("does not load or execute v2 modules"),
        "{message}"
    );
    assert!(
        message.contains("cannot fall back to legacy client execution"),
        "{message}"
    );
    assert!(!message.contains("type errors"), "{message}");
}

#[test]
fn unsupported_and_dynamic_versions_have_authored_diagnostics() {
    for (argument, expected) in [
        (
            "3",
            "version 2 is the migration target, but this build does not load it yet",
        ),
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
fn explicit_v2_precedes_monty_evaluation() {
    let message =
        load_error("POISON = 1 / 0\nSPEC = module_spec(tasks=[], spec_version=2)\nSPEC\n");

    assert!(
        message.contains("does not load or execute v2 modules"),
        "{message}"
    );
    assert!(!message.contains("failed to evaluate"), "{message}");
}
