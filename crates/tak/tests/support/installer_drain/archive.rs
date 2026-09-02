use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn create(root: &Path) -> PathBuf {
    let package = root.join("package");
    fs::create_dir_all(&package).expect("create package");
    write_executable(&package.join("tak"), tak_script());
    write_executable(&package.join("takd"), takd_script());
    let archive = root.join("release.tar.gz");
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(&archive)
        .args(["-C"])
        .arg(&package)
        .args(["tak", "takd"])
        .status()
        .expect("create release archive");
    assert!(status.success());
    archive
}

pub(super) fn install_build_artifacts(target: &Path) {
    let release = target.join("release");
    fs::create_dir_all(&release).expect("create release target");
    write_executable(&release.join("tak"), tak_script());
    write_executable(&release.join("takd"), takd_script());
}

pub(super) fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write executable");
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod executable");
}

fn tak_script() -> &'static str {
    "#!/bin/sh\nprintf 'new-tak\\n'\n"
}

fn takd_script() -> &'static str {
    r#"#!/bin/sh
set -eu
if [ "${1:-}" = update ] && [ "${2:-}" = --legacy-drain-check ]; then
  state_root="${4:-${XDG_STATE_HOME:-$HOME/.local/state}/takd}"
  if [ -f "$state_root/agent.sqlite" ]; then
    echo 'active legacy attempts must finish before replacing tak/takd binaries' >&2
    exit 42
  fi
  exit 0
fi
if [ "${1:-}" = init ]; then
  root="${XDG_CONFIG_HOME:-$HOME/.config}/takd"
  mkdir -p "$root"
  printf 'display_name = "fixture"\n' > "$root/agent.toml"
fi
printf 'new-takd\n'
"#
}
