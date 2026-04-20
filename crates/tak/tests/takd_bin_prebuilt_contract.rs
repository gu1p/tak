use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::support;

use support::binary::BinaryPathInputs;
use support::tor_smoke::resolve_takd_bin;

#[test]
fn takd_bin_reuses_prebuilt_target_binary_without_spawning_cargo() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let target_dir = temp.path().join("target");
    let binary = target_dir.join("debug").join("takd");
    fs::create_dir_all(binary.parent().expect("debug dir"))?;
    fs::write(&binary, b"fake takd")?;

    assert_eq!(
        resolve_takd_bin(&BinaryPathInputs {
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
