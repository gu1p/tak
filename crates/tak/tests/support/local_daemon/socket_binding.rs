use std::path::{Path, PathBuf};

pub(super) struct SocketBinding {
    server_path: PathBuf,
}

impl SocketBinding {
    pub(super) fn new(requested_path: &Path) -> Self {
        secure_parent(requested_path);
        #[cfg(unix)]
        if let Some(relative_path) = short_checkout_relative_path(requested_path) {
            return Self {
                server_path: relative_path,
            };
        }
        Self {
            server_path: requested_path.to_path_buf(),
        }
    }

    pub(super) fn server_path(&self) -> &Path {
        &self.server_path
    }
}

fn secure_parent(socket_path: &Path) {
    let Some(parent) = socket_path.parent() else {
        return;
    };
    std::fs::create_dir_all(parent).expect("create local daemon socket directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .expect("secure local daemon socket directory");
    }
}

#[cfg(unix)]
fn path_exceeds_unix_limit(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().len() > 103
}

#[cfg(unix)]
fn short_checkout_relative_path(requested_path: &Path) -> Option<PathBuf> {
    if !path_exceeds_unix_limit(requested_path) {
        return None;
    }
    let current_dir = std::env::current_dir().ok()?;
    let relative = relative_path(requested_path, &current_dir)
        .or_else(|| canonical_relative_path(requested_path, &current_dir))?;
    (!path_exceeds_unix_limit(&relative)).then_some(relative)
}

#[cfg(unix)]
fn canonical_relative_path(requested_path: &Path, current_dir: &Path) -> Option<PathBuf> {
    let current_dir = current_dir.canonicalize().ok()?;
    let parent = requested_path.parent()?.canonicalize().ok()?;
    let requested_path = parent.join(requested_path.file_name()?);
    relative_path(&requested_path, &current_dir)
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
