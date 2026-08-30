use std::collections::BTreeMap;

use super::workspace::{Preparation, prepare};
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};

#[test]
fn preparation_discards_a_partial_workspace_left_before_start() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive_path = temp.path().join("workspace.tar");
    write_archive(&archive_path);
    let attempt_root = temp.path().join("attempt");
    let stale = attempt_root.join("workspace/stale");
    std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
    std::fs::write(&stale, b"partial").unwrap();
    let snapshot = LocalExecutionSnapshot {
        archive_path,
        attempt_root,
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Private,
        overlays: Vec::new(),
    };

    let Preparation::Execute { workspace_root, .. } = prepare(snapshot).unwrap() else {
        panic!("partial pre-start workspace should be prepared again")
    };
    assert!(!workspace_root.join("stale").exists());
    assert_eq!(
        std::fs::read(workspace_root.join("fresh")).unwrap(),
        b"complete"
    );
}

#[test]
fn shared_preparations_reuse_one_root_and_preserve_undeclared_writes() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive_path = temp.path().join("workspace.tar");
    write_archive(&archive_path);
    let shared = temp.path().join("shared");
    let first = snapshot(&archive_path, temp.path().join("attempt-1"), &shared);
    let Preparation::Execute { workspace_root, .. } = prepare(first).unwrap() else {
        panic!("first shared workspace should execute")
    };
    assert_eq!(workspace_root, shared.join("data"));
    std::fs::write(workspace_root.join("generated"), b"producer").unwrap();

    let second = snapshot(&archive_path, temp.path().join("attempt-2"), &shared);
    let Preparation::Execute { workspace_root, .. } = prepare(second).unwrap() else {
        panic!("second shared workspace should execute")
    };
    assert_eq!(
        std::fs::read(workspace_root.join("generated")).unwrap(),
        b"producer"
    );
}

fn snapshot(
    archive_path: &std::path::Path,
    attempt_root: std::path::PathBuf,
    shared: &std::path::Path,
) -> LocalExecutionSnapshot {
    LocalExecutionSnapshot {
        archive_path: archive_path.to_owned(),
        attempt_root,
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Shared(shared.to_owned()),
        overlays: Vec::new(),
    }
}

fn write_archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    let mut header = tar::Header::new_gnu();
    header.set_size(8);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "fresh", &b"complete"[..])
        .unwrap();
    archive.finish().unwrap();
}
