use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn v2_loader_does_not_ship_v1_entrypoints_or_evaluators() {
    let lib = fs::read_to_string(crate_root().join("src/lib.rs")).unwrap();
    let loader = fs::read_to_string(crate_root().join("src/loader/mod.rs")).unwrap();
    for removed in ["discover_tasks_files", "evaluate_named_policy_decision"] {
        assert!(!lib.contains(removed), "crate still exports {removed}");
        assert!(!loader.contains(removed), "loader still exports {removed}");
    }

    for removed in [
        "src/loader/prelude.py",
        "src/loader/dsl_stubs.pyi",
        "src/loader/module_eval.rs",
        "src/loader/execution_policy_registry.rs",
        "src/loader/execution_policy_resolution.rs",
    ] {
        assert!(
            !crate_root().join(removed).exists(),
            "legacy file remains: {removed}"
        );
    }
}
