use std::path::PathBuf;

use super::launcher::{persist_request, read_request, spawn_reaper};
use crate::daemon::scheduler::DispatchCommand;

#[test]
fn wrapper_request_is_private_and_contains_only_dispatch_identity() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let command = command();
    let path = persist_request(temp.path(), PathBuf::from("state.sqlite"), &command).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.windows(6).any(|window| window == b"secret"));
    let request = read_request(&path).unwrap();
    assert_eq!(request.db_path, PathBuf::from("state.sqlite"));
    assert_eq!(request.command, command);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn persisting_the_same_request_is_idempotent() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let command = command();
    let first = persist_request(temp.path(), PathBuf::from("state.sqlite"), &command).unwrap();
    let second = persist_request(temp.path(), PathBuf::from("state.sqlite"), &command).unwrap();
    assert_eq!(first, second);
}

#[test]
fn detached_wrapper_is_reaped_after_it_exits() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let marker = temp.path().join("reaped");
    let child = std::process::Command::new("/bin/sh")
        .args(["-c", "printf done > \"$1\"", "reaper-test"])
        .arg(&marker)
        .spawn()
        .unwrap();

    spawn_reaper(child).unwrap().join().unwrap();
    assert_eq!(std::fs::read(marker).unwrap(), b"done");
}

fn command() -> DispatchCommand {
    DispatchCommand {
        run_id: "run".into(),
        job_id: "job".into(),
        node_id: "local".into(),
        authored_attempt: 1,
        dispatch_generation: 2,
        fencing_token: "fence".into(),
    }
}
