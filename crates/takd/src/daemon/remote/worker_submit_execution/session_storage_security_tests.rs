use std::os::unix::fs::symlink;

use super::*;

#[test]
fn refresh_does_not_follow_a_session_parent_swapped_to_a_symlink() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    let sessions = root.join(SESSION_WORKSPACES_DIR_NAME);
    let moved_sessions = root.join("moved-sessions");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&sessions).expect("create session parent");
    fs::create_dir(&outside).expect("create outside directory");
    let outside_baseline = backdate(&outside);
    let session = RemoteWorkerSession {
        key: "run-session".into(),
        reuse: RemoteWorkerSessionReuse::ShareWorkspace,
    };

    let error = refresh_session_storage_parent_with(&root, Some(&session), |path| {
        fs::rename(path, &moved_sessions)?;
        symlink(&outside, path)?;
        open_session_storage_parent(path)
    })
    .expect_err("swapped symlink must not be followed");

    assert!(format!("{error:#}").contains("open session storage parent"));
    assert_eq!(modified(&outside), outside_baseline);
}

fn backdate(path: &Path) -> SystemTime {
    fs::File::open(path)
        .expect("open outside directory")
        .set_modified(SystemTime::UNIX_EPOCH)
        .expect("backdate outside directory");
    modified(path)
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}
