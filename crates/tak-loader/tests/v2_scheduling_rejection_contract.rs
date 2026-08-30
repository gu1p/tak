use std::fs;

use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn unsupported_scheduling_options_are_rejected_instead_of_dropped() {
    for (field, expression, expected) in [
        (
            "retry",
            "retry(attempts=2, on_exit=[7])",
            "on_exit filtering is not active",
        ),
        (
            "retry",
            "retry(attempts=2, backoff=exp_jitter())",
            "exponential jitter retry is not active",
        ),
    ] {
        let source = format!(
            "SPEC = module_spec(spec_version=2, tasks=[task('check', {field}={expression})])\nSPEC\n"
        );
        let error = load_error(&source);
        assert!(error.contains(expected), "{error}");
    }
}

fn load_error(source: &str) -> String {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(temp.path().join("TASKS.py"), source).unwrap();
    inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .unwrap_err()
        .to_string()
}
