#![allow(dead_code)]

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Clone)]
pub struct TakdBinaryPathInputs {
    pub cargo_bin_exe: Option<PathBuf>,
    pub test_bin_override: Option<PathBuf>,
    pub cargo_target_dir: Option<PathBuf>,
    pub current_exe: Option<PathBuf>,
    pub workspace_root: PathBuf,
}

fn profile_dir(inputs: &TakdBinaryPathInputs) -> PathBuf {
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

pub fn resolve_takd_bin(inputs: &TakdBinaryPathInputs) -> PathBuf {
    if let Some(path) = inputs.cargo_bin_exe.clone() {
        return path;
    }

    if let Some(path) = inputs.test_bin_override.clone() {
        return path;
    }

    let binary = profile_dir(inputs).join(format!("takd{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "missing takd binary at {}; prebuild it or set TAK_TEST_TAKD_BIN",
        binary.display()
    );
    binary
}

pub fn takd_bin() -> PathBuf {
    resolve_takd_bin(&TakdBinaryPathInputs {
        cargo_bin_exe: std::env::var_os("CARGO_BIN_EXE_takd").map(PathBuf::from),
        test_bin_override: std::env::var_os("TAK_TEST_TAKD_BIN").map(PathBuf::from),
        cargo_target_dir: std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
        current_exe: std::env::current_exe().ok(),
        workspace_root: workspace_root(),
    })
}

pub fn roots(temp: &Path) -> (PathBuf, PathBuf) {
    (temp.join("config"), temp.join("state"))
}
