//! Contract for Cargo doctest policy on the `tak-exec` library crate.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

#[test]
fn tak_exec_library_disables_cargo_doctests() -> Result<()> {
    let manifest = fs::read_to_string(manifest_path()).context("read tak-exec Cargo.toml")?;
    let value: toml::Value = manifest.parse().context("parse tak-exec Cargo.toml")?;
    let doctest = value
        .get("lib")
        .and_then(|lib| lib.get("doctest"))
        .and_then(toml::Value::as_bool);

    assert_eq!(
        doctest,
        Some(false),
        "expected [lib] doctest = false in crates/tak-exec/Cargo.toml"
    );
    Ok(())
}
