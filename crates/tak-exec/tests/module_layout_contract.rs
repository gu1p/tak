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

#[test]
fn tak_exec_engine_uses_explicit_imports_without_unused_import_shims() -> Result<()> {
    assert_no_unused_import_allow("src/lib.rs");
    assert_no_unused_import_allow("src/engine/mod.rs");

    for entry in fs::read_dir(crate_root().join("src/engine")).context("read src/engine")? {
        let entry = entry.context("read engine entry")?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let relative_path = path
            .strip_prefix(crate_root())
            .expect("engine path under crate root")
            .display()
            .to_string();
        let source = read(&relative_path)?;
        assert!(
            !source.contains("use super::*;"),
            "{relative_path} should use explicit imports instead of super::*",
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

fn assert_no_unused_import_allow(relative_path: &str) {
    let source = read(relative_path).expect("source");
    assert!(
        !source.contains("#[allow(unused_imports)]"),
        "{relative_path} should not suppress unused-import warnings to support hidden import hubs",
    );
}
