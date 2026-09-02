use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

use super::preflight;

#[test]
fn checkout_change_matching_daemon_output_is_still_a_conflict() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    std::fs::write(temp.path().join("result.txt"), "daemon").unwrap();
    let submitted = manifest(vec![file("result.txt", b"submitted")]);
    let outputs = manifest(vec![file("result.txt", b"daemon")]);

    let error = preflight(temp.path(), &submitted, &outputs)
        .unwrap_err()
        .to_string();

    assert!(error.contains("result.txt"), "{error}");
}

#[test]
fn file_and_symlink_replacements_report_every_changed_descendant() {
    for replacement in [
        file("dist", b"daemon"),
        WorkspaceEntry::symlink("dist", "target.txt").unwrap(),
    ] {
        assert_directory_replacement_conflicts(replacement);
    }
}

fn assert_directory_replacement_conflicts(replacement: WorkspaceEntry) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("dist/nested")).unwrap();
    std::fs::write(root.join("dist/one.txt"), "local-one").unwrap();
    std::fs::write(root.join("dist/nested/two.txt"), "local-two").unwrap();
    std::fs::write(root.join("dist/local.txt"), "local-new").unwrap();
    let submitted = manifest(vec![
        WorkspaceEntry::directory("dist").unwrap(),
        WorkspaceEntry::directory("dist/nested").unwrap(),
        file("dist/one.txt", b"submitted-one"),
        file("dist/nested/two.txt", b"submitted-two"),
    ]);
    let outputs = manifest(vec![replacement]);

    let error = preflight(root, &submitted, &outputs)
        .unwrap_err()
        .to_string();

    for path in ["dist/one.txt", "dist/nested/two.txt", "dist/local.txt"] {
        assert!(error.contains(path), "missing {path}: {error}");
    }
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
