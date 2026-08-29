use std::fs;

use tak_loader::evaluate_named_policy_decision;

#[test]
fn trailing_tuple_expression_does_not_become_a_selected_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose(ctx):\n  return Decision.local()\nmodule_spec(tasks=[]),\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("a trailing tuple is not a module spec");
}

#[test]
fn trailing_dictionary_expression_does_not_become_a_selected_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose(ctx):\n  return Decision.local()\n{'spec_version': 2}\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("an arbitrary trailing dictionary is not a selected module spec");
}

#[test]
fn selected_module_expression_is_evaluated_once() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "STATE = [0]\ndef selected():\n  STATE[0] += 1\n  return module_spec(tasks=[])\ndef choose(ctx):\n  if STATE[0] != 1:\n    return 1 / 0\n  return Decision.local()\nselected()\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("selected module expression should run exactly once");
}

#[test]
fn invalid_runtime_version_stops_before_policy_invocation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    for version in ["True", "-1", "'2'", "4294967296"] {
        fs::write(
            &tasks_file,
            format!(
                "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = {version}\ndef choose(ctx):\n  return 1 / 0\nSPEC\n"
            ),
        )
        .expect("write tasks");

        let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
            .expect_err("invalid runtime version should fail before the policy")
            .to_string();
        assert!(
            message.contains("invalid evaluated module version"),
            "version {version}: {message}"
        );
        assert!(!message.contains("division by zero"), "{message}");
    }
}
