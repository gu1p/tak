use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn tak_exec_worker_runtime_uses_explicit_module_boundaries() -> Result<()> {
    assert_no_include_assembly("src/lib.rs", &["container_runtime/", "step_runner/"]);
    assert_no_include_assembly("src/container_runtime/mod.rs", &["container_runtime/"]);
    assert_no_include_assembly("src/step_runner/mod.rs", &["step_runner/"]);

    for relative_path in [
        "src/container_runtime/mod.rs",
        "src/execution_types.rs",
        "src/remote_worker.rs",
        "src/runtime_metadata.rs",
        "src/step_runner/mod.rs",
        "src/worker_runtime.rs",
    ] {
        assert!(
            crate_root().join(relative_path).is_file(),
            "expected {relative_path} to exist as a real module root"
        );
    }
    Ok(())
}

#[test]
fn tak_exec_worker_modules_do_not_hide_unused_imports() -> Result<()> {
    for relative_path in [
        "src/lib.rs",
        "src/container_runtime/mod.rs",
        "src/step_runner/mod.rs",
        "src/worker_runtime.rs",
    ] {
        let source = read(relative_path)?;
        assert!(
            !source.contains("#[allow(unused_imports)]"),
            "{relative_path} should not suppress unused imports"
        );
    }
    Ok(())
}

fn read(relative_path: &str) -> Result<String> {
    fs::read_to_string(crate_root().join(relative_path))
        .with_context(|| format!("read {relative_path}"))
}

fn assert_no_include_assembly(relative_path: &str, fragments: &[&str]) {
    let source = read(relative_path).expect("source");
    for fragment in fragments {
        assert!(
            !source.contains(&format!("include!(\"{fragment}")),
            "{relative_path} should not include-expand {fragment} modules"
        );
    }
}
