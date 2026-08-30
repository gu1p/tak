use std::collections::BTreeMap;

use super::workspace::{Preparation, prepare};
use crate::daemon::run_store::execution::LocalExecutionSnapshot;

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
