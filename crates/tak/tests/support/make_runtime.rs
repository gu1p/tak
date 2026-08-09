#![allow(dead_code)] // shared by test binaries that each exercise a subset of helpers

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Result;

pub fn install_fake_make(root: &Path, script: &str) -> Result<String> {
    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let fake_make = bin.join("make");
    fs::write(&fake_make, script)?;
    fs::set_permissions(&fake_make, fs::Permissions::from_mode(0o755))?;

    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    Ok(std::env::join_paths(paths)?.to_string_lossy().into_owned())
}
