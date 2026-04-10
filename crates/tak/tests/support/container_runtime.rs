#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn simulated_container_runtime_env(root: &Path) -> BTreeMap<String, String> {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".into(), "other".into());
    env.insert(
        "PATH".into(),
        format!(
            "{}:{}",
            bin_root.display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );
    env
}

fn install_fake_docker(bin_root: &Path) {
    fs::create_dir_all(bin_root).expect("create fake docker bin");
    let docker = bin_root.join("docker");
    fs::write(
        &docker,
        "#!/usr/bin/env bash\nif [[ \"$1\" == \"--version\" ]]; then exit 0; fi\nexit 1\n",
    )
    .expect("write fake docker");
    let mut permissions = fs::metadata(&docker).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&docker, permissions).expect("chmod fake docker");
}
