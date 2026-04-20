use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::support;

use support::binary::{BinaryPathInputs, resolve_binary_path};

#[test]
fn tak_bin_prefers_explicit_test_binary_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let explicit = temp.path().join("tak-explicit");
    fs::write(&explicit, b"fake tak")?;

    assert_eq!(
        resolve_binary_path(
            &BinaryPathInputs {
                cargo_bin_exe: None,
                test_bin_override: Some(explicit.clone()),
                cargo_target_dir: None,
                current_exe: None,
                workspace_root: PathBuf::from("/workspace"),
            },
            "tak",
        ),
        explicit
    );
    Ok(())
}
