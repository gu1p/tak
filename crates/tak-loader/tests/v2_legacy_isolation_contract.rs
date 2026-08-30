use std::fs;

use tak_loader::{
    LoadOptions, discover_tasks_files, evaluate_named_policy_decision, load_workspace,
};

#[test]
fn every_legacy_loader_entrypoint_fails_closed_for_v2() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "SPEC = module_spec(spec_version=2, tasks=[])\ndef choose(ctx):\n  return 1 / 0\nSPEC\n",
    )
    .expect("write tasks");

    let load = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("v2 cannot produce WorkspaceSpec")
        .to_string();
    assert!(load.contains("loaded and validated"), "{load}");
    assert!(
        load.contains("no legacy WorkspaceSpec was produced"),
        "{load}"
    );

    let discover = discover_tasks_files(temp.path(), &LoadOptions::default())
        .expect_err("v2 cannot enter legacy discovery")
        .to_string();
    assert!(discover.contains("cannot enter legacy"), "{discover}");

    let policy = evaluate_named_policy_decision(&tasks_file, "//", "choose")
        .expect_err("v2 cannot enter legacy policy evaluation")
        .to_string();
    assert!(policy.contains("cannot enter legacy"), "{policy}");
    assert!(
        !policy.contains("division by zero"),
        "poison evaluated: {policy}"
    );
}
