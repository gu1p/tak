use std::collections::BTreeMap;
use std::sync::{Arc, Barrier};

use super::workspace::{Preparation, prepare};
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};

#[test]
fn simultaneous_shared_preparations_publish_one_complete_root() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive = Arc::new(temp.path().join("workspace.tar"));
    write_archive(&archive);
    let shared = Arc::new(temp.path().join("shared"));
    let barrier = Arc::new(Barrier::new(3));
    let workers = (0..2)
        .map(|index| {
            let archive = Arc::clone(&archive);
            let shared = Arc::clone(&shared);
            let barrier = Arc::clone(&barrier);
            let attempt = temp.path().join(format!("attempt-{index}"));
            std::thread::spawn(move || {
                barrier.wait();
                let snapshot = LocalExecutionSnapshot {
                    archive_path: archive.as_ref().clone(),
                    attempt_root: attempt,
                    tasks: Vec::new(),
                    environment: BTreeMap::new(),
                    workspace: LocalWorkspace::Shared(shared.as_ref().clone()),
                };
                let Preparation::Execute { workspace_root, .. } = prepare(snapshot).unwrap() else {
                    panic!("shared attempt should execute")
                };
                workspace_root
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let roots = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(roots, vec![shared.join("data"), shared.join("data")]);
    assert_eq!(std::fs::read(shared.join("data/base")).unwrap(), b"data");
    assert!(!std::fs::read_dir(temp.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("shared-")
    }));
}

fn write_archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "base", &b"data"[..])
        .unwrap();
    archive.finish().unwrap();
}
