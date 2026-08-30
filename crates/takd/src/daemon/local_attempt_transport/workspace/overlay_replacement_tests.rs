use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntry;

use crate::daemon::run_store::output_artifacts::OutputOverlay;

#[test]
fn declared_directory_replaces_base_descendants_before_overlaying_children() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let root = temp.path().join("workspace");
    std::fs::create_dir_all(root.join("dist")).unwrap();
    std::fs::write(root.join("dist/stale.txt"), "stale").unwrap();
    let blob = temp.path().join("fresh.blob");
    std::fs::write(&blob, "fresh").unwrap();
    let overlays = [
        OutputOverlay {
            entry: WorkspaceEntry::directory("dist").unwrap(),
            blob_path: None,
        },
        OutputOverlay {
            entry: WorkspaceEntry::file(
                "dist/fresh.txt",
                false,
                5,
                &format!("{:x}", Sha256::digest(b"fresh")),
            )
            .unwrap(),
            blob_path: Some(blob),
        },
    ];

    super::overlays::apply(&root, &overlays).unwrap();

    assert!(!root.join("dist/stale.txt").exists());
    assert_eq!(
        std::fs::read(root.join("dist/fresh.txt")).unwrap(),
        b"fresh"
    );
}
