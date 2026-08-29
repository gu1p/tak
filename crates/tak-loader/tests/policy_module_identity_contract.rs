use std::fs;

use tak_loader::evaluate_named_policy_decision;

#[test]
fn unrelated_dictionary_mutation_cannot_hide_a_selected_module_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "SAVED = module_spec\nmodule_spec = 0\nSPEC = SAVED(tasks=[])\nSPEC['spec_version'] = 2\nSPEC['__tak_kind'] = 'other'\ndef choose(ctx):\n  return Decision.local()\nSPEC\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("selected module identity must survive unrelated dictionary mutation")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
}

#[test]
fn a_forged_dictionary_tag_cannot_create_a_selected_module_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose(ctx):\n  return Decision.local()\n{'__tak_kind': 'module_spec', 'spec_version': 2}\n",
    )
    .expect("write tasks");

    evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect("an authored dictionary tag alone cannot forge module identity");
}

#[test]
fn complete_manual_module_payloads_receive_the_version_backstop() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose(ctx):\n  return Decision.local()\n{'spec_version': 2, 'project_id': None, 'tasks': [], 'limiters': [], 'queues': [], 'exclude': [], 'includes': [], 'defaults': {}}\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("a complete module payload must be version checked")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
}

#[test]
fn removing_a_defaulted_field_cannot_hide_a_selected_module_version() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC.pop('defaults')\ndef choose(ctx):\n  return 1 / 0\nSPEC\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("a selected module must remain versioned after field removal")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(!message.contains("division by zero"), "{message}");
}

#[test]
fn a_complete_copied_module_with_ignored_metadata_keeps_the_version_backstop() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "ORIGINAL = module_spec(tasks=[])\nSPEC = ORIGINAL.copy()\nSPEC['spec_version'] = 2\nSPEC['ignored_metadata'] = 'kept'\ndef choose(ctx):\n  return 1 / 0\nSPEC\n",
    )
    .expect("write tasks");

    let message = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("a complete copied module payload must be version checked")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(!message.contains("division by zero"), "{message}");
}
