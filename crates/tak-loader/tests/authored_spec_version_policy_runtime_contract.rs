use std::fs;

use tak_loader::{LoadOptions, evaluate_named_policy_decision, load_workspace};

#[test]
fn named_policy_evaluation_rejects_a_runtime_mutated_module_version() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\ndef choose(ctx):\n  return 1 / 0\nSPEC\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("runtime version mutation should fail before returning a decision")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(
        message.contains("legacy bootstrap requires version 1"),
        "{message}"
    );
}

#[test]
fn named_policy_checks_the_final_indirect_module_result() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def build():\n  spec = module_spec(tasks=[])\n  spec['spec_version'] = 2\n  return spec\nSPEC = build()\ndef choose(ctx):\n  return Decision.local()\nSPEC\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("the final indirect module result should be version checked")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(message.contains("no direct module_spec"), "{message}");
    assert!(!message.contains("TASKS.py:1:1"), "{message}");
}

#[test]
fn named_policy_validates_only_the_selected_final_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def stale():\n  return module_spec(tasks=[])\nOLD = stale()\nOLD['spec_version'] = 2\nSPEC = module_spec(tasks=[])\ndef choose(ctx):\n  return Decision.local()\nSPEC\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("an unused module object must not replace the selected final module");
}

#[test]
fn policy_only_source_may_end_with_a_non_module_expression() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose(ctx):\n  return Decision.local()\nchoose\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("a trailing policy expression is not a selected module spec");
}

#[test]
fn explicit_v2_refusal_does_not_depend_on_type_checking() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[], spec_version=2)\nSPEC\n",
    )
    .expect("write tasks");
    let options = LoadOptions {
        enable_type_check: false,
        ..LoadOptions::default()
    };

    let message = load_workspace(temp.path(), &options)
        .expect_err("explicit v2 must stop before submission")
        .to_string();
    assert!(message.contains("loaded and validated"), "{message}");
    assert!(message.contains("no client executor fallback"), "{message}");
    assert!(!message.contains("failed to evaluate"), "{message}");
}
