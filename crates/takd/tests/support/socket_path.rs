use std::path::{Path, PathBuf};

#[cfg(unix)]
const PORTABLE_UNIX_SOCKET_PATH_LIMIT: usize = 103;

pub fn bind_path(requested: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        if path_len(requested) <= PORTABLE_UNIX_SOCKET_PATH_LIMIT {
            return requested.to_path_buf();
        }
        let Ok(current_dir) = std::env::current_dir() else {
            return requested.to_path_buf();
        };
        relative_path(requested, &current_dir)
            .filter(|path| path_len(path) <= PORTABLE_UNIX_SOCKET_PATH_LIMIT)
            .unwrap_or_else(|| requested.to_path_buf())
    }
    #[cfg(not(unix))]
    requested.to_path_buf()
}

#[cfg(unix)]
fn path_len(path: &Path) -> usize {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().len()
}

#[cfg(unix)]
fn relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let target = target.components().collect::<Vec<_>>();
    let base = base.components().collect::<Vec<_>>();
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(target, base)| target == base)
        .count();
    if common == 0 {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in &base[common..] {
        match component {
            Component::Normal(_) => relative.push(".."),
            Component::CurDir => {}
            _ => return None,
        }
    }
    for component in &target[common..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}
