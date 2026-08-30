use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

use super::preflight;

#[test]
fn every_destination_changed_since_submission_is_reported() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let root = temp.path();
    std::fs::write(root.join("existing.txt"), "local-existing").unwrap();
    std::fs::write(root.join("new.txt"), "local-new").unwrap();
    let submitted = manifest(vec![file("existing.txt", b"submitted")]);
    let outputs = manifest(vec![
        file("existing.txt", b"daemon-existing"),
        file("new.txt", b"daemon-new"),
        file("safe.txt", b"daemon-safe"),
    ]);

    let error = preflight(root, &submitted, &outputs)
        .unwrap_err()
        .to_string();

    assert!(error.contains("existing.txt"), "{error}");
    assert!(error.contains("new.txt"), "{error}");
    assert!(!error.contains("safe.txt"), "{error}");
}

#[test]
fn directory_output_detects_local_descendants_but_accepts_the_submitted_tree() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("dist")).unwrap();
    std::fs::write(root.join("dist/keep.txt"), "submitted").unwrap();
    let submitted = manifest(vec![
        WorkspaceEntry::directory("dist").unwrap(),
        file("dist/keep.txt", b"submitted"),
    ]);
    let outputs = manifest(vec![
        WorkspaceEntry::directory("dist").unwrap(),
        file("dist/keep.txt", b"daemon"),
        file("dist/result.txt", b"result"),
    ]);
    preflight(root, &submitted, &outputs).unwrap();
    std::fs::write(root.join("dist/local.txt"), "local").unwrap();

    let error = preflight(root, &submitted, &outputs)
        .unwrap_err()
        .to_string();

    assert!(error.contains("dist/local.txt"), "{error}");
}

fn file(path: &str, contents: &[u8]) -> WorkspaceEntry {
    WorkspaceEntry::file(
        path,
        false,
        contents.len() as u64,
        &format!("{:x}", Sha256::digest(contents)),
    )
    .unwrap()
}

fn manifest(entries: Vec<WorkspaceEntry>) -> WorkspaceManifest {
    WorkspaceManifest::new(entries).unwrap()
}
