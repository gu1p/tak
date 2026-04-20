use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::support;

use support::binary::BinaryPathInputs;
use support::tor_smoke::resolve_takd_bin_with_bootstrap;

#[test]
fn takd_bin_bootstraps_missing_target_binary_once() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let target_dir = temp.path().join("target");
    let binary = target_dir
        .join("debug")
        .join(format!("takd{}", std::env::consts::EXE_SUFFIX));
    let bootstraps = Cell::new(0usize);

    let resolved = resolve_takd_bin_with_bootstrap(
        &BinaryPathInputs {
            cargo_bin_exe: None,
            test_bin_override: None,
            cargo_target_dir: Some(target_dir),
            current_exe: None,
            workspace_root: PathBuf::from("/workspace"),
        },
        |_, missing_binary| {
            bootstraps.set(bootstraps.get() + 1);
            fs::create_dir_all(
                missing_binary
                    .parent()
                    .expect("bootstrap target should have a parent directory"),
            )
            .expect("create bootstrap target dir");
            fs::write(missing_binary, b"bootstrapped takd").expect("write bootstrapped takd");
        },
    );

    assert_eq!(resolved, binary);
    assert_eq!(
        bootstraps.get(),
        1,
        "missing takd should bootstrap exactly once"
    );
    assert!(
        binary.is_file(),
        "bootstrap should materialize the missing takd binary"
    );
    Ok(())
}
