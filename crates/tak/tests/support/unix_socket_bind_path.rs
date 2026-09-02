#![allow(dead_code)]

use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const MAX_UNIX_SOCKET_PATH_BYTES: usize = 103;

pub fn short_bind_path(requested: &Path) -> PathBuf {
    if unix_path_fits(requested) {
        return requested.to_path_buf();
    }
    let Ok(current) = std::env::current_dir() else {
        return requested.to_path_buf();
    };
    let Some(relative) = relative_path(requested, &current) else {
        return requested.to_path_buf();
    };
    if unix_path_fits(&relative) {
        relative
    } else {
        requested.to_path_buf()
    }
}

fn unix_path_fits(path: &Path) -> bool {
    path.as_os_str().as_bytes().len() <= MAX_UNIX_SOCKET_PATH_BYTES
}

fn relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = target.components().collect::<Vec<_>>();
    let base = base.components().collect::<Vec<_>>();
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(target, base)| target == base)
        .count();
    if common == 0 {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in &base[common..] {
        match component {
            Component::Normal(_) => relative.push(".."),
            Component::CurDir => {}
            _ => return None,
        }
    }
    for component in &target[common..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}
