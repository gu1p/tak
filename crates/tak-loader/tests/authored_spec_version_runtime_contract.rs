use std::fs;

use tak_loader::{LoadOptions, evaluate_named_policy_decision, load_workspace};

#[test]
fn included_module_reports_its_own_explicit_version_gate() {
    for (version, expected) in [(1, "Migration summary"), (2, "cannot enter legacy")] {
        let temp = tempfile::tempdir().expect("tempdir");
        let child = temp.path().join("apps/web");
        fs::create_dir_all(&child).expect("mkdir child");
        fs::write(
            temp.path().join("TASKS.py"),
            "SPEC = module_spec(tasks=[], includes=[path('apps/web')])\nSPEC\n",
        )
        .expect("write root");
        fs::write(
            child.join("TASKS.py"),
            format!("SPEC = module_spec(tasks=[], spec_version={version})\nSPEC\n"),
        )
        .expect("write child");

        let message = load_workspace(temp.path(), &LoadOptions::default())
            .expect_err("child version gate should fail")
            .to_string();
        assert!(message.contains("apps/web/TASKS.py:1:"), "{message}");
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn evaluated_payload_keeps_a_version_one_backstop() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC\n",
    )
    .expect("write tasks");
    let options = LoadOptions {
        enable_type_check: false,
        ..LoadOptions::default()
    };

    let message = load_workspace(temp.path(), &options)
        .expect_err("runtime version should fail")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(
        message.contains("authored version was omitted"),
        "{message}"
    );
    assert!(
        message.contains("legacy bootstrap requires version 1"),
        "{message}"
    );
}

#[test]
fn named_policy_evaluation_stops_at_the_authored_version_gate() {
    for (version, expected) in [(1, "Migration summary"), (2, "cannot enter legacy")] {
        let temp = tempfile::tempdir().expect("tempdir");
        let tasks_file = temp.path().join("TASKS.py");
        fs::write(
            &tasks_file,
            format!(
                "BROKEN = Runtime.Host()\nSPEC = module_spec(tasks=[], spec_version={version})\ndef choose(ctx):\n  return Decision.local()\nSPEC\n"
            ),
        )
        .expect("write tasks");

        let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
            .expect_err("authored version should fail before policy evaluation")
            .to_string();
        assert!(message.contains(expected), "{message}");
        assert!(!message.contains("`Runtime` was replaced"), "{message}");
    }
}

#[test]
fn indirect_module_declaration_remains_a_legacy_bootstrap() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "def build():\n  return module_spec(tasks=[task('check', steps=[cmd('true')])])\nSPEC = build()\nSPEC\n",
    )
    .expect("write tasks");

    let workspace = load_workspace(temp.path(), &LoadOptions::default()).expect("legacy load");
    assert_eq!(workspace.tasks.len(), 1);
}
