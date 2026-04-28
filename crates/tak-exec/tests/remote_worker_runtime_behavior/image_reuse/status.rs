use std::path::PathBuf;

use tak_exec::image_cache_status;

use crate::support::env_lock;

#[test]
fn image_cache_status_matches_relative_db_path_to_current_filesystem() {
    let _env_lock = env_lock();
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let target_root = workspace_root.join("target");
    std::fs::create_dir_all(&target_root).expect("create target root");
    let temp = tempfile::tempdir_in(&target_root).expect("target tempdir");
    let _cwd = CurrentDirGuard::enter(workspace_root.clone());

    let relative_path = temp
        .path()
        .strip_prefix(&workspace_root)
        .expect("relative tempdir")
        .join("agent.sqlite");
    let absolute_path = workspace_root.join(&relative_path);

    let relative = image_cache_status(&relative_path, 1, 10.0, 1).expect("relative cache status");
    let absolute = image_cache_status(&absolute_path, 1, 10.0, 1).expect("absolute cache status");

    assert_eq!(
        relative.filesystem_total_bytes, absolute.filesystem_total_bytes,
        "relative cache db paths should inspect the same filesystem as absolute paths"
    );
    assert_eq!(relative.free_floor_bytes, absolute.free_floor_bytes);
}

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn enter(next: PathBuf) -> Self {
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
