use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::support;

use support::binary::BinaryPathInputs;
use support::tor_smoke::resolve_takd_bin;

#[test]
fn takd_bin_prefers_explicit_test_binary_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let explicit = temp.path().join("takd-explicit");
    fs::write(&explicit, b"fake takd")?;

    assert_eq!(
        resolve_takd_bin(&BinaryPathInputs {
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
