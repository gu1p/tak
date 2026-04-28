#![cfg(test)]

use std::path::{Path, PathBuf};

use super::{DiskCandidate, available_bytes_for_path_with_disks};

#[test]
fn relative_state_root_is_canonicalized_before_matching_disk_mounts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let _cwd = CurrentDirGuard::enter(temp.path());
    let fallback_available_bytes = 1;
    let state_root_available_bytes = 42;

    let selected = available_bytes_for_path_with_disks(
        Path::new("state"),
        &[
            DiskCandidate {
                mount_point: PathBuf::from("/fallback"),
                available_bytes: fallback_available_bytes,
            },
            DiskCandidate {
                mount_point: state_root,
                available_bytes: state_root_available_bytes,
            },
        ],
    )
    .expect("available bytes");

    assert_eq!(selected, state_root_available_bytes);
}

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn enter(next: &Path) -> Self {
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(next).expect("set current dir");
        Self { previous }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).expect("restore current dir");
    }
}
