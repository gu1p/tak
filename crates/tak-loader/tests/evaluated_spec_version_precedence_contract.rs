use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn evaluated_version_rejection_precedes_malformed_module_fields() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC['tasks'] = 42\nSPEC[17] = 'unrelated'\nSPEC\n",
    )
    .expect("write tasks");
    let options = LoadOptions {
        enable_type_check: false,
        ..LoadOptions::default()
    };

    let message = load_workspace(temp.path(), &options)
        .expect_err("v2 must be rejected before unrelated payload decoding")
        .to_string();
    assert!(message.contains("evaluated spec_version=2"), "{message}");
    assert!(!message.contains("invalid module spec"), "{message}");
}
