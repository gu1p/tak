use std::collections::BTreeMap;

use tak_core::v2::JobContextManifest;

use super::workspace::{Preparation, prepare};
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};

#[test]
fn private_jobs_reuse_an_immutable_base_and_discard_undeclared_writes() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archives = temp.path().join("workspaces");
    std::fs::create_dir(&archives).unwrap();
    let archive = archives.join("fingerprint.tar");
    write_archive(&archive);

    let first = root(prepare(snapshot(&archive, temp.path().join("attempt-1"))).unwrap());
    std::fs::write(first.join("seed.txt"), b"changed").unwrap();
    std::fs::write(first.join("undeclared.txt"), b"leak").unwrap();
    std::fs::remove_file(&archive).unwrap();

    let second = root(prepare(snapshot(&archive, temp.path().join("attempt-2"))).unwrap());
    assert_eq!(std::fs::read(second.join("seed.txt")).unwrap(), b"base");
    assert!(!second.join("undeclared.txt").exists());
    let base = temp.path().join("workspace-bases/fingerprint/data");
    assert_eq!(std::fs::read(base.join("seed.txt")).unwrap(), b"base");
}

fn root(preparation: Preparation) -> std::path::PathBuf {
    let Preparation::Execute { workspace_root, .. } = preparation else {
        panic!("fresh private attempt should execute")
    };
    workspace_root
}

fn snapshot(archive: &std::path::Path, attempt_root: std::path::PathBuf) -> LocalExecutionSnapshot {
    LocalExecutionSnapshot {
        archive_path: archive.to_owned(),
        attempt_root,
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Private,
        overlays: Vec::new(),
        context_manifest: JobContextManifest {
            paths: vec!["seed.txt".into()],
        },
    }
}

fn write_archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "seed.txt", &b"base"[..])
        .unwrap();
    archive.finish().unwrap();
}
