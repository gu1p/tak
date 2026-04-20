#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use super::binary::{BinaryPathInputs, profile_dir, workspace_root};

pub fn resolve_takd_bin_with_bootstrap<F>(inputs: &BinaryPathInputs, bootstrap: F) -> PathBuf
where
    F: FnOnce(&BinaryPathInputs, &Path),
{
    if let Some(path) = inputs.cargo_bin_exe.clone() {
        return path;
    }
    if let Some(path) = inputs.test_bin_override.clone() {
        return path;
    }

    let binary = profile_dir(inputs).join(format!("takd{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        bootstrap(inputs, &binary);
    }
    assert!(
        binary.is_file(),
        "missing takd binary at {}; prebuild it or set the test override",
        binary.display()
    );
    binary
}

fn bootstrap_takd_binary(inputs: &BinaryPathInputs, expected_binary: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = StdCommand::new(cargo);
    command
        .current_dir(&inputs.workspace_root)
        .args(["build", "-p", "takd", "--bin", "takd"]);
    if let Some(target_dir) = inputs.cargo_target_dir.as_ref() {
        command.env("CARGO_TARGET_DIR", target_dir);
    }

    let status = command
        .status()
        .expect("build takd binary for tak integration test");
    assert!(status.success(), "building takd binary should succeed");
    assert!(
        expected_binary.is_file(),
        "missing takd binary at {} after bootstrap build",
        expected_binary.display()
    );
}

pub fn resolve_takd_bin(inputs: &BinaryPathInputs) -> PathBuf {
    resolve_takd_bin_with_bootstrap(inputs, bootstrap_takd_binary)
}

pub fn takd_bin() -> PathBuf {
    resolve_takd_bin(&BinaryPathInputs {
        cargo_bin_exe: std::env::var_os("CARGO_BIN_EXE_takd").map(PathBuf::from),
        test_bin_override: std::env::var_os("TAK_TEST_TAKD_BIN").map(PathBuf::from),
        cargo_target_dir: std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
        current_exe: std::env::current_exe().ok(),
        workspace_root: workspace_root(),
    })
}
