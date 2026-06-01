#![cfg(test)]

use sha2::{Digest, Sha256};

use super::workspace_stage::write_zip_snapshot_hashed;

#[test]
fn staged_workspace_archive_is_file_backed_and_hashed_on_write() {
    let temp = tempfile::tempdir().expect("tempdir");
    let staged_root = temp.path().join("files");
    std::fs::create_dir_all(&staged_root).expect("files dir");
    std::fs::write(staged_root.join("input.txt"), b"stream me").expect("input");
    let archive_path = temp.path().join("workspace.zip");

    let (byte_len, digest) =
        write_zip_snapshot_hashed(&staged_root, &archive_path).expect("zip archive");

    let archive = std::fs::read(&archive_path).expect("archive");
    assert_eq!(byte_len, archive.len() as u64);
    assert_eq!(digest, format!("{:x}", Sha256::digest(&archive)));
    assert!(!archive.is_empty());
}
