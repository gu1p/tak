//! Contract for Cargo doctest policy on the `tak-runner` library crate.

use std::fs;
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

#[test]
fn tak_runner_library_disables_cargo_doctests() {
    let manifest = fs::read_to_string(manifest_path()).expect("read tak-runner Cargo.toml");
    assert!(
        manifest.contains("[lib]") && manifest.contains("doctest = false"),
        "expected [lib] doctest = false in crates/tak-runner/Cargo.toml"
    );
}
