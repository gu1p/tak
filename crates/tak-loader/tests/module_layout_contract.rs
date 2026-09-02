use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative_path: &str) -> String {
    fs::read_to_string(crate_root().join(relative_path)).expect("source")
}

#[test]
fn tak_loader_v2_uses_real_module_boundaries() {
    assert_no_include_assembly("src/lib.rs", &["loader/"]);
    assert_no_include_assembly("src/loader/mod.rs", &["loader/"]);

    for relative_path in [
        "src/loader/mod.rs",
        "src/loader/authored_root.rs",
        "src/loader/load_options.rs",
        "src/loader/v2_includes.rs",
        "src/loader/v2_module_eval.rs",
        "src/loader/v2_wire.rs",
        "src/loader/v2_wire_conversion.rs",
        "src/loader/v2_workspace_view.rs",
        "src/loader/workspace_discovery.rs",
        "src/loader/workspace_load_and_policy_eval.rs",
    ] {
        assert!(
            crate_root().join(relative_path).is_file(),
            "expected {relative_path} to exist as a real module file"
        );
    }

    assert!(
        !crate_root()
            .join("tests/runtime_resolution_contract")
            .exists(),
        "stale unregistered runtime-resolution module directory remains"
    );
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
