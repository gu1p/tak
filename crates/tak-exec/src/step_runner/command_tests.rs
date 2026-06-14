#![cfg(test)]
use std::path::{Path, PathBuf};

use super::resolve_cwd;

#[test]
fn resolve_cwd_returns_workspace_root_when_unset() {
    let root = Path::new("/work/space");
    assert_eq!(resolve_cwd(root, &None), PathBuf::from("/work/space"));
}

#[test]
fn resolve_cwd_joins_relative_path_onto_workspace_root() {
    let root = Path::new("/work/space");
    assert_eq!(
        resolve_cwd(root, &Some("sub/dir".to_string())),
        PathBuf::from("/work/space/sub/dir")
    );
}

#[test]
fn resolve_cwd_passes_absolute_path_through() {
    let root = Path::new("/work/space");
    assert_eq!(
        resolve_cwd(root, &Some("/abs/elsewhere".to_string())),
        PathBuf::from("/abs/elsewhere")
    );
}
