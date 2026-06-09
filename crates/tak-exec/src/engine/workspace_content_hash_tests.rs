#![cfg(test)]

use super::workspace_content_hash::workspace_upload_identity;
use std::fs;
use std::path::Path;
use tak_core::model::CurrentStateSpec;

fn seed_tree(root: &Path, payload: &[u8]) {
    fs::write(root.join("top.txt"), payload).expect("top file");
    fs::create_dir_all(root.join("nested")).expect("nested dir");
    fs::write(root.join("nested/inner.txt"), b"inner").expect("inner file");
}

fn content_hash(root: &Path, state: &CurrentStateSpec) -> String {
    workspace_upload_identity(root, state)
        .expect("upload identity")
        .content_hash
}

#[test]
fn identical_content_hashes_identically_and_is_content_sensitive() {
    let state = CurrentStateSpec::default();

    let a = tempfile::tempdir().expect("tempdir a");
    seed_tree(a.path(), b"hello");
    let hash_a = content_hash(a.path(), &state);

    // Deterministic across repeated calls (no wall-clock / ordering dependence).
    assert_eq!(hash_a, content_hash(a.path(), &state));

    // Byte-identical content in a different directory hashes the same.
    let b = tempfile::tempdir().expect("tempdir b");
    seed_tree(b.path(), b"hello");
    assert_eq!(hash_a, content_hash(b.path(), &state));

    // Changing a file's contents (same paths) must change the hash — this is what a
    // paths-only manifest hash would miss.
    let c = tempfile::tempdir().expect("tempdir c");
    seed_tree(c.path(), b"HELLO");
    assert_ne!(hash_a, content_hash(c.path(), &state));
}
