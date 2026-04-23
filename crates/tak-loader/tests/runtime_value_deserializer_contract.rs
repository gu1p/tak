use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn rejects_tuple_module_output_as_unsupported_runtime_value() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), "SPEC = (1, 2)\nSPEC\n").expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    let message = err.to_string();
    assert!(
        message.contains("unsupported Monty runtime value"),
        "missing Monty runtime rejection: {message:#}"
    );
    assert!(
        message.contains("tuple"),
        "missing tuple runtime detail: {message:#}"
    );
}
