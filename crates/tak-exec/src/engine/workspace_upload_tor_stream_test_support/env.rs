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

impl Drop for TakdSocketEnv {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => unsafe { std::env::set_var("TAKD_SOCKET", previous) },
            None => unsafe { std::env::remove_var("TAKD_SOCKET") },
        }
    }
}
