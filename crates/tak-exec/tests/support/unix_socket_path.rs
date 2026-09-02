use std::path::{Path, PathBuf};

const PORTABLE_UNIX_SOCKET_PATH_LIMIT: usize = 103;

pub fn short_socket_bind_path(socket_path: &Path) -> PathBuf {
    if path_len(socket_path) <= PORTABLE_UNIX_SOCKET_PATH_LIMIT {
        return socket_path.to_path_buf();
    }
    let Ok(current) = std::env::current_dir() else {
        return socket_path.to_path_buf();
    };
    relative_path(socket_path, &current)
        .filter(|path| path_len(path) <= PORTABLE_UNIX_SOCKET_PATH_LIMIT)
        .unwrap_or_else(|| socket_path.to_path_buf())
}

fn path_len(path: &Path) -> usize {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().len()
}

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
