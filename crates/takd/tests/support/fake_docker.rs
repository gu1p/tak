#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn install_fake_docker(bin_root: &Path) {
    fs::create_dir_all(bin_root).expect("create fake bin dir");
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
