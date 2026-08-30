use std::fs;

use super::identity::scope_key_for_path;

#[test]
fn worktree_owner_keys_are_canonical_stable_and_distinct() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();

    let direct = scope_key_for_path(&first).unwrap();
    let equivalent = scope_key_for_path(&first.join(".")).unwrap();
    let different = scope_key_for_path(&second).unwrap();
    assert_eq!(direct, equivalent);
    assert_ne!(direct, different);
    assert!(direct.starts_with("worktree-") && direct.len() == 73);
}
