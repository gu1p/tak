//! Contract for workspace Cargo test manifests.

use std::fs;
use std::path::Path;

use anyhow::Result;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

fn read_repo_file(path: &str) -> Result<String> {
    Ok(fs::read_to_string(repo_root().join(path))?)
}

#[test]
fn workspace_tested_crates_use_explicit_suite_targets() -> Result<()> {
    for manifest in [
        "crates/tak-core/Cargo.toml",
        "crates/tak-loader/Cargo.toml",
        "crates/tak-proto/Cargo.toml",
        "crates/tak-runner/Cargo.toml",
        "crates/tak-exec/Cargo.toml",
        "crates/takd/Cargo.toml",
        "crates/tak/Cargo.toml",
    ] {
        let contents = read_repo_file(manifest)?;
        assert!(contents.contains("autotests = false"));
        assert!(contents.contains("[[test]]\nname = \"suite\"\npath = \"tests/mod.rs\""));
    }
    Ok(())
}

#[test]
fn tak_manifest_keeps_named_contract_test_targets() -> Result<()> {
    let contents = read_repo_file("crates/tak/Cargo.toml")?;

    for expected in [
        "[[test]]\nname = \"doctest_contract\"\npath = \"tests/doctest_contract.rs\"",
        concat!(
            "[[test]]\n",
            "name = \"examples_matrix_contract\"\n",
            "path = \"tests/examples_matrix_contract.rs\""
        ),
    ] {
        assert!(contents.contains(expected));
    }
    Ok(())
}

#[test]
fn tak_and_takd_binary_targets_do_not_build_test_harnesses() -> Result<()> {
    for (manifest, bin_name) in [
        ("crates/tak/Cargo.toml", "tak"),
        ("crates/takd/Cargo.toml", "takd"),
    ] {
        let contents = read_repo_file(manifest)?;
        assert!(contents.contains("[[bin]]"));
        assert!(contents.contains(&format!("name = \"{bin_name}\"")));
        assert!(contents.contains("path = \"src/main.rs\""));
        assert!(contents.contains("test = false"));
    }
    Ok(())
}
