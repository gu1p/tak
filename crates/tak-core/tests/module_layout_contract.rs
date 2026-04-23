use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative_path: &str) -> String {
    fs::read_to_string(crate_root().join(relative_path)).expect("source")
}

#[test]
fn tak_core_model_uses_real_module_boundaries_instead_of_include_assembly() {
    assert_no_include_assembly("src/model.rs", &["model/"]);

    for relative_path in [
        "src/model/container_runtime_limits.rs",
        "src/model/container_runtime_normalization.rs",
        "src/model/container_runtime_types.rs",
        "src/model/container_runtime_validation.rs",
        "src/model/context_manifest.rs",
        "src/model/current_state_manifest.rs",
        "src/model/execution_policy.rs",
        "src/model/limiter_retry.rs",
        "src/model/module_spec.rs",
        "src/model/path_anchor.rs",
        "src/model/relative_path.rs",
        "src/model/remote_config.rs",
        "src/model/resolved_workspace.rs",
        "src/model/task_identity.rs",
    ] {
        assert!(
            crate_root().join(relative_path).is_file(),
            "expected {relative_path} to exist as a real module file"
        );
    }
}

fn assert_no_include_assembly(relative_path: &str, fragments: &[&str]) {
    let source = read(relative_path);
    for fragment in fragments {
        assert!(
            !source.contains(&format!("include!(\"{fragment}")),
            "{relative_path} should not include-expand {fragment} modules",
        );
    }
}
