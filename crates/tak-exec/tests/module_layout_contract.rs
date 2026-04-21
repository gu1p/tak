use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative_path: &str) -> Result<String> {
    fs::read_to_string(crate_root().join(relative_path))
        .with_context(|| format!("read {relative_path}"))
}

#[test]
fn tak_exec_uses_real_module_boundaries_instead_of_include_assembly() -> Result<()> {
    assert_no_include_assembly("src/lib.rs", &["engine/"]);
    assert_no_include_assembly("src/container_runtime/mod.rs", &["container_runtime/"]);
    assert_no_include_assembly("src/lease_client/mod.rs", &["lease_client/"]);
    assert_no_include_assembly(
        "src/remote_protocol_codec/mod.rs",
        &["remote_protocol_codec/"],
    );
    assert_no_include_assembly("src/step_runner/mod.rs", &["step_runner/"]);

    for relative_path in [
        "src/engine/mod.rs",
        "src/container_runtime/mod.rs",
        "src/lease_client/mod.rs",
        "src/remote_protocol_codec/mod.rs",
        "src/step_runner/mod.rs",
    ] {
        assert!(
            crate_root().join(relative_path).is_file(),
            "expected {relative_path} to exist as a real module root"
        );
    }

    Ok(())
}

fn assert_no_include_assembly(relative_path: &str, fragments: &[&str]) {
    let source = read(relative_path).expect("source");
    for fragment in fragments {
        assert!(
            !source.contains(&format!("include!(\"{fragment}")),
            "{relative_path} should not include-expand {fragment} modules",
        );
    }
}
