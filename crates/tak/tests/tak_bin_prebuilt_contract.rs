use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::support;

use support::binary::{BinaryPathInputs, resolve_binary_path};

#[test]
fn tak_bin_reuses_prebuilt_target_binary_without_spawning_cargo() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let target_dir = temp.path().join("target");
    let binary = target_dir.join("debug").join("tak");
    fs::create_dir_all(binary.parent().expect("debug dir"))?;
    fs::write(&binary, b"fake tak")?;

    assert_eq!(
        resolve_binary_path(
            &BinaryPathInputs {
                cargo_bin_exe: None,
                test_bin_override: None,
                cargo_target_dir: Some(target_dir),
                current_exe: None,
                workspace_root: PathBuf::from("/workspace"),
            },
            "tak",
        ),
        binary
    );
    Ok(())
}
