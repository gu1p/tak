use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use super::{default_remote_execution_root_base, explicit_remote_execution_root_base};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RemoteExecRootCacheKey {
    pub(super) explicit_root: Option<PathBuf>,
    pub(super) temp_dir: PathBuf,
    pub(super) docker_host: Option<String>,
    pub(super) podman_socket: Option<String>,
    pub(super) runtime_dir: Option<String>,
    pub(super) uid: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct RemoteExecRootCacheEntry {
    pub(super) key: RemoteExecRootCacheKey,
    pub(super) selected_root: PathBuf,
    pub(super) probed: bool,
}

pub(super) fn current_remote_execution_root_entry() -> RemoteExecRootCacheEntry {
    let key = current_remote_exec_root_cache_key();
    let mut guard = remote_exec_root_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache_entry(&mut guard, key).clone()
}

pub(super) fn current_remote_exec_root_cache_key() -> RemoteExecRootCacheKey {
    RemoteExecRootCacheKey {
        explicit_root: explicit_remote_execution_root_base(),
        temp_dir: env::temp_dir(),
        docker_host: env::var("DOCKER_HOST").ok(),
        podman_socket: env::var("TAK_PODMAN_SOCKET").ok(),
        runtime_dir: env::var("XDG_RUNTIME_DIR").ok(),
        uid: env::var("UID").ok(),
    }
}

pub(super) fn update_remote_execution_root_entry(
    key: RemoteExecRootCacheKey,
    selected_root: PathBuf,
) {
    let mut guard = remote_exec_root_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry = cache_entry(&mut guard, key);
    entry.selected_root = selected_root;
    entry.probed = true;
}

fn remote_exec_root_cache() -> &'static Mutex<Option<RemoteExecRootCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<RemoteExecRootCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn cache_entry(
    guard: &mut Option<RemoteExecRootCacheEntry>,
    key: RemoteExecRootCacheKey,
) -> &mut RemoteExecRootCacheEntry {
    if guard.as_ref().map(|entry| entry.key != key).unwrap_or(true) {
        let selected_root = key
            .explicit_root
            .clone()
            .unwrap_or_else(|| default_remote_execution_root_base(&key));
        *guard = Some(RemoteExecRootCacheEntry {
            key,
            selected_root,
            probed: false,
        });
    }

    let entry = guard.as_mut().expect("cache entry");
    if entry.key.explicit_root.is_some() {
        entry.probed = true;
    }
    entry
}
