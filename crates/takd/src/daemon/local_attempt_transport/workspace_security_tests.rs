use std::collections::BTreeMap;

use super::workspace::prepare;
use crate::daemon::run_store::execution::{LocalExecutionSnapshot, LocalWorkspace};

#[cfg(unix)]
#[test]
fn shared_preparation_rejects_a_symlinked_ready_marker() {
    use std::os::unix::fs::symlink;

    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive = temp.path().join("workspace.tar");
    write_archive(&archive);
    let shared = temp.path().join("shared");
    std::fs::create_dir_all(shared.join("data")).unwrap();
    let outside = temp.path().join("outside");
    std::fs::write(&outside, b"forged").unwrap();
    symlink(&outside, shared.join("ready")).unwrap();

    assert!(prepare(snapshot(&archive, temp.path().join("attempt"), &shared)).is_err());
}

#[cfg(unix)]
#[test]
fn shared_preparation_publishes_owner_only_directories_and_marker() {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let archive = temp.path().join("workspace.tar");
    write_archive(&archive);
    let shared = temp.path().join("shared");
    prepare(snapshot(&archive, temp.path().join("attempt"), &shared)).unwrap();

    assert_eq!(
        std::fs::metadata(&shared).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(shared.join("data"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(shared.join("ready"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

fn snapshot(
    archive: &std::path::Path,
    attempt: std::path::PathBuf,
    shared: &std::path::Path,
) -> LocalExecutionSnapshot {
    LocalExecutionSnapshot {
        archive_path: archive.to_owned(),
        attempt_root: attempt,
        tasks: Vec::new(),
        environment: BTreeMap::new(),
        workspace: LocalWorkspace::Shared(shared.to_owned()),
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
        .append_data(&mut header, "base", &b"data"[..])
        .unwrap();
    archive.finish().unwrap();
}
