use std::ffi::OsString;
use std::path::Path;

pub(super) struct TakdSocketEnv {
    previous: Option<OsString>,
}

impl TakdSocketEnv {
    pub(super) fn set(socket_path: &Path) -> Self {
        let previous = std::env::var_os("TAKD_SOCKET");
        unsafe { std::env::set_var("TAKD_SOCKET", socket_path) };
        Self { previous }
    }
}

pub(crate) struct EnvVarGuard {
    name: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    pub(crate) fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(name);
        unsafe { std::env::set_var(name, value) };
        Self { name, previous }
    }

    pub(crate) fn remove(name: &'static str) -> Self {
        let previous = std::env::var_os(name);
        unsafe { std::env::remove_var(name) };
        Self { name, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => unsafe { std::env::set_var(self.name, previous) },
            None => unsafe { std::env::remove_var(self.name) },
        }
    }
}

impl Drop for TakdSocketEnv {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => unsafe { std::env::set_var("TAKD_SOCKET", previous) },
            None => unsafe { std::env::remove_var("TAKD_SOCKET") },
        }
    }
}
