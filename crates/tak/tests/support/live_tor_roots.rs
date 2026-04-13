#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub struct LiveTorRoots {
    pub server_config_root: PathBuf,
    pub server_state_root: PathBuf,
    pub client_config_root: PathBuf,
    pub client_state_root: PathBuf,
    client_data_home_poison: PathBuf,
    client_cache_home_poison: PathBuf,
}

impl LiveTorRoots {
    pub fn new(base: &Path) -> Self {
        Self {
            server_config_root: base.join("server-config"),
            server_state_root: base.join("server-state"),
            client_config_root: base.join("client-config"),
            client_state_root: base.join("client-state"),
            client_data_home_poison: base.join("client-data-home-poison"),
            client_cache_home_poison: base.join("client-cache-home-poison"),
        }
    }

    pub fn service_log_path(&self) -> PathBuf {
        self.server_state_root.join("service.log")
    }

    pub fn prepare_poisoned_client_ambient_dirs(&self) {
        std::fs::write(
            &self.client_data_home_poison,
            "poison ambient arti data home\n",
        )
        .expect("write poisoned XDG_DATA_HOME file");
        std::fs::write(
            &self.client_cache_home_poison,
            "poison ambient arti cache home\n",
        )
        .expect("write poisoned XDG_CACHE_HOME file");
    }

    pub fn poisoned_client_data_home(&self) -> &Path {
        &self.client_data_home_poison
    }

    pub fn poisoned_client_cache_home(&self) -> &Path {
        &self.client_cache_home_poison
    }
}
