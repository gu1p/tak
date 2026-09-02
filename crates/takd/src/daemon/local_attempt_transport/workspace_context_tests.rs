use std::collections::BTreeMap;

use tak_core::v2::JobContextManifest;

use super::workspace::{Preparation, prepare};
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};

#[test]
fn private_local_workspace_contains_only_its_job_context_manifest() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive_path = temp.path().join("workspace.tar");
    write_archive(&archive_path);
    let snapshot = LocalExecutionSnapshot {
        archive_path,
        attempt_root: temp.path().join("attempt"),
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Private,
        overlays: Vec::new(),
        context_manifest: JobContextManifest {
            paths: vec!["keep.txt".into()],
        },
    };

    let Preparation::Execute { workspace_root, .. } = prepare(snapshot).unwrap() else {
        panic!("attempt should execute")
    };
    assert!(workspace_root.join("keep.txt").is_file());
    assert!(!workspace_root.join("drop.txt").exists());
}

fn write_archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    for name in ["keep.txt", "drop.txt"] {
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, name, &b"data"[..])
            .unwrap();
    }
    archive.finish().unwrap();
}
