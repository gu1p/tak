use tak_core::v2::OutputSelector;

use super::path_cache::{PathCache, Publication};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn only_the_first_writer_of_a_generation_publishes() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let cache = PathCache::new(temp.path().join("cache"), vec![path("build")]).unwrap();
    let first_root = temp.path().join("first");
    let second_root = temp.path().join("second");
    std::fs::create_dir_all(&first_root).unwrap();
    std::fs::create_dir_all(&second_root).unwrap();
    let first = cache.restore_into(&first_root).unwrap();
    let second = cache.restore_into(&second_root).unwrap();
    write(&first_root, "first");
    write(&second_root, "second");

    assert_eq!(
        cache.publish_from(&first_root, first).unwrap(),
        Publication::Published
    );
    assert_eq!(
        cache.publish_from(&second_root, second).unwrap(),
        Publication::GenerationChanged
    );
    let restored = temp.path().join("restored");
    std::fs::create_dir_all(&restored).unwrap();
    cache.restore_into(&restored).unwrap();
    assert_eq!(
        std::fs::read(restored.join("build/value")).unwrap(),
        b"first"
    );
}

#[cfg(unix)]
#[test]
fn restored_executable_does_not_mutate_the_published_generation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let cache = PathCache::new(temp.path().join("cache"), vec![path("build/tool")]).unwrap();
    let publisher = temp.path().join("publisher");
    std::fs::create_dir_all(publisher.join("build")).unwrap();
    let snapshot = cache.restore_into(&publisher).unwrap();
    let executable = publisher.join("build/tool");
    std::fs::write(&executable, b"published").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        cache.publish_from(&publisher, snapshot).unwrap(),
        Publication::Published
    );

    let restored = temp.path().join("restored");
    std::fs::create_dir_all(&restored).unwrap();
    cache.restore_into(&restored).unwrap();
    let restored_executable = restored.join("build/tool");
    assert!(
        std::fs::symlink_metadata(&restored_executable)
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(std::fs::read(&restored_executable).unwrap(), b"published");
    assert_ne!(mode(&restored_executable) & 0o111, 0);

    std::fs::write(&restored_executable, b"mutated").unwrap();
    std::fs::set_permissions(&restored_executable, std::fs::Permissions::from_mode(0o644)).unwrap();
    cache.restore_into(&restored).unwrap();
    assert_eq!(std::fs::read(&restored_executable).unwrap(), b"published");
    assert_ne!(mode(&restored_executable) & 0o111, 0);
}

fn write(root: &std::path::Path, value: &str) {
    std::fs::create_dir_all(root.join("build")).unwrap();
    std::fs::write(root.join("build/value"), value).unwrap();
}

fn path(value: &str) -> OutputSelector {
    OutputSelector::Path {
        value: value.into(),
    }
}

#[cfg(unix)]
fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode()
}
