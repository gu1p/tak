#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn takd_bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin!("takd").to_path_buf()
}

pub fn roots(temp: &Path) -> (PathBuf, PathBuf) {
    (temp.join("config"), temp.join("state"))
}
