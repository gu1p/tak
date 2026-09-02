use std::collections::BTreeMap;

use super::workspace::prepare;
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};
use tak_core::v2::JobContextManifest;

#[cfg(unix)]
#[test]
fn shared_preparation_rejects_a_symlinked_storage_parent() {
    use std::os::unix::fs::symlink;

    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive = temp.path().join("workspace.tar");
    let file = std::fs::File::create(&archive).unwrap();
    tar::Builder::new(file).finish().unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let parent = temp.path().join("parent");
    symlink(&outside, &parent).unwrap();
    let snapshot = LocalExecutionSnapshot {
        archive_path: archive,
        attempt_root: temp.path().join("attempt"),
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Shared(parent.join("shared")),
        overlays: Vec::new(),
        context_manifest: JobContextManifest { paths: Vec::new() },
    };

    assert!(prepare(snapshot).is_err());
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
}
