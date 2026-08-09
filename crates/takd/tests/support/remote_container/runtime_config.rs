use std::{env, path::Path};

use crate::support::env::EnvGuard;
use crate::support::fake_docker::install_fake_docker;
use crate::support::runtime_config::{self, RuntimeConfigBuilder};

pub fn configure_fake_docker_env(
    root: &Path,
    socket_path: &Path,
    env_guard: &mut EnvGuard,
) -> RuntimeConfigBuilder {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);
    env_guard.set(
        "PATH",
        format!(
            "{}:{}",
            bin_root.display(),
            env::var("PATH").unwrap_or_default()
        ),
    );
    let docker_host = format!("unix://{}", socket_path.display());
    env_guard.set("DOCKER_HOST", &docker_host);
    runtime_config::builder().with_docker_host(docker_host)
}
