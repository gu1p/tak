//! Contract for `takd` CLI binary resolution inside the crate test binary.

use crate::support;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use support::cli::{TakdBinaryPathInputs, resolve_takd_bin};

#[test]
fn takd_cli_binary_prefers_explicit_test_binary_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let explicit = temp.path().join("takd-explicit");
    fs::write(&explicit, b"fake takd")?;

    assert_eq!(
        resolve_takd_bin(&TakdBinaryPathInputs {
            cargo_bin_exe: None,
            test_bin_override: Some(explicit.clone()),
            cargo_target_dir: None,
            current_exe: None,
            workspace_root: PathBuf::from("/workspace"),
        }),
        explicit
    );
    Ok(())
}

#[test]
fn takd_cli_binary_reuses_prebuilt_target_binary() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let target_dir = temp.path().join("target");
    let binary = target_dir.join("debug").join("takd");
    fs::create_dir_all(binary.parent().expect("debug dir"))?;
    fs::write(&binary, b"fake takd")?;

    assert_eq!(
        resolve_takd_bin(&TakdBinaryPathInputs {
            cargo_bin_exe: None,
            test_bin_override: None,
            cargo_target_dir: Some(target_dir),
            current_exe: None,
            workspace_root: PathBuf::from("/workspace"),
        }),
        binary
    );
    Ok(())
}
