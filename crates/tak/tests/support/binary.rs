#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Clone)]
pub struct BinaryPathInputs {
    pub cargo_bin_exe: Option<PathBuf>,
    pub test_bin_override: Option<PathBuf>,
    pub cargo_target_dir: Option<PathBuf>,
    pub current_exe: Option<PathBuf>,
    pub workspace_root: PathBuf,
}

pub fn profile_dir(inputs: &BinaryPathInputs) -> PathBuf {
    if let Some(path) = inputs.cargo_target_dir.clone() {
        let root = if path.is_absolute() {
            path
        } else {
            inputs.workspace_root.join(path)
        };
        return root.join("debug");
    }

    inputs
        .current_exe
        .clone()
        .and_then(|path| {
            path.parent()
                .and_then(|parent| parent.parent())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| inputs.workspace_root.join("target").join("debug"))
}

pub fn resolve_binary_path(inputs: &BinaryPathInputs, binary_name: &str) -> PathBuf {
    if let Some(path) = inputs.cargo_bin_exe.clone() {
        return path;
    }
    if let Some(path) = inputs.test_bin_override.clone() {
        return path;
    }

    let binary = profile_dir(inputs).join(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "missing {binary_name} binary at {}; prebuild it or set the test override",
        binary.display()
    );
    binary
}

pub fn tak_bin() -> PathBuf {
    resolve_binary_path(
        &BinaryPathInputs {
            cargo_bin_exe: std::env::var_os("CARGO_BIN_EXE_tak").map(PathBuf::from),
            test_bin_override: std::env::var_os("TAK_TEST_TAK_BIN").map(PathBuf::from),
            cargo_target_dir: std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
            current_exe: std::env::current_exe().ok(),
            workspace_root: workspace_root(),
        },
        "tak",
    )
}
