//! Contract tests for the local source installer script.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

/// Verifies local installer builds and installs both binaries into `~/.local/bin` by default.
#[test]
fn installs_tak_and_takd_into_default_local_bin() {
    let fixture = InstallerFixture::new();
    fixture.run("v1", "/bin/zsh", PathMode::WithoutInstallDirInPath);

    let install_dir = fixture.home_dir().join(".local/bin");
    assert!(install_dir.join("tak").exists(), "tak should be installed");
    assert!(
        install_dir.join("takd").exists(),
        "takd should be installed"
    );

    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("v1"),
        "installed tak should come from latest local build output"
    );
}

/// Verifies rerunning installer replaces existing binaries with newly built outputs.
#[test]
fn rerun_replaces_existing_binaries_with_new_build() {
    let fixture = InstallerFixture::new();
    fixture.run("old", "/bin/bash", PathMode::WithoutInstallDirInPath);
    fixture.run("new", "/bin/bash", PathMode::WithoutInstallDirInPath);

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("new"),
        "new build should replace old binary"
    );
    assert!(
        !tak_body.contains("old"),
        "old build content should not remain after replacement"
    );
}

/// Verifies installer falls back to `~/bin` when `~/.local/bin` cannot be created.
#[test]
fn falls_back_to_home_bin_when_dot_local_bin_is_unavailable() {
    let fixture = InstallerFixture::new();
    fs::write(fixture.home_dir().join(".local"), "blocker").expect("write blocker file");

    fixture.run("fallback", "/bin/bash", PathMode::WithoutInstallDirInPath);

    assert!(
        fixture.home_dir().join("bin/tak").exists(),
        "tak should be installed to ~/bin fallback"
    );
    assert!(
        fixture.home_dir().join("bin/takd").exists(),
        "takd should be installed to ~/bin fallback"
    );
}

/// Verifies installer adds one PATH line to active shell rc file and does not duplicate it.
#[test]
fn appends_path_to_active_shell_rc_once() {
    let fixture = InstallerFixture::new();
    let rc = fixture.home_dir().join(".zshrc");

    fixture.run("v1", "/bin/zsh", PathMode::WithoutInstallDirInPath);
    fixture.run("v2", "/bin/zsh", PathMode::WithoutInstallDirInPath);

    let rc_content = fs::read_to_string(&rc).expect("zshrc should exist");
    let expected = format!(
        "export PATH=\"{}:$PATH\"",
        fixture.home_dir().join(".local/bin").display()
    );
    let occurrences = rc_content.lines().filter(|line| *line == expected).count();
    assert_eq!(occurrences, 1, "path export should be appended once");
}

/// Verifies installer does not touch rc file when install directory is already in current PATH.
#[test]
fn does_not_edit_shell_rc_when_install_dir_already_in_path() {
    let fixture = InstallerFixture::new();
    let rc = fixture.home_dir().join(".bashrc");
    fs::write(&rc, "# keep-me\n").expect("seed bashrc");

    fixture.run("v1", "/bin/bash", PathMode::WithInstallDirInPath);

    let rc_content = fs::read_to_string(&rc).expect("bashrc should exist");
    assert_eq!(rc_content, "# keep-me\n", "bashrc should not be modified");
}

/// Verifies installer resolves build artifacts from `CARGO_TARGET_DIR` when it is configured.
#[test]
fn installs_from_custom_cargo_target_dir() {
    let fixture = InstallerFixture::new();
    let custom_target = fixture.home_dir().join("custom-target");

    fixture.run_with_target_dir(
        "custom",
        "/bin/bash",
        PathMode::WithoutInstallDirInPath,
        &custom_target,
    );

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("custom"),
        "installer should install binary emitted under CARGO_TARGET_DIR"
    );
}

/// Verifies installer follows Cargo metadata target-directory when env override is not set.
#[test]
fn installs_from_metadata_target_directory_without_env_override() {
    let fixture = InstallerFixture::new();
    let metadata_target = fixture.home_dir().join("metadata-target");

    fixture.run_with_metadata_target_no_env(
        "meta",
        "/bin/bash",
        PathMode::WithoutInstallDirInPath,
        &metadata_target,
    );

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("meta"),
        "installer should use cargo metadata target_directory"
    );
}

/// PATH mode to simulate whether install dir is already discoverable in the running shell.
enum PathMode {
    WithInstallDirInPath,
    WithoutInstallDirInPath,
}

/// Isolated test fixture for invoking `install-locally.sh` with a fake cargo build.
struct InstallerFixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    fake_bin: PathBuf,
    home: PathBuf,
}

impl InstallerFixture {
    /// Creates an isolated workspace containing the installer script and a fake cargo binary.
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let fake_bin = temp.path().join("fake-bin");
        let home = temp.path().join("home");

        fs::create_dir_all(&workspace).expect("create workspace");
        fs::create_dir_all(&fake_bin).expect("create fake-bin");
        fs::create_dir_all(&home).expect("create home");

        let script_src = repo_root().join("install-locally.sh");
        let script_dst = workspace.join("install-locally.sh");
        fs::copy(&script_src, &script_dst).unwrap_or_else(|err| {
            panic!(
                "failed to copy installer script {} -> {}: {err}",
                script_src.display(),
                script_dst.display()
            )
        });

        let cargo_path = fake_bin.join("cargo");
        fs::write(&cargo_path, fake_cargo_script()).expect("write fake cargo");
        let mut perms = fs::metadata(&cargo_path)
            .expect("cargo metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&cargo_path, perms).expect("chmod fake cargo");

        Self {
            _temp: temp,
            workspace,
            fake_bin,
            home,
        }
    }

    /// Runs the installer script with the provided fake build tag and shell identity.
    fn run(&self, build_tag: &str, shell: &str, path_mode: PathMode) {
        self.run_with_optional_target_dir(build_tag, shell, path_mode, None);
    }

    /// Runs the installer script with a custom `CARGO_TARGET_DIR`.
    fn run_with_target_dir(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        target_dir: &Path,
    ) {
        self.run_with_optional_target_dir(build_tag, shell, path_mode, Some(target_dir));
    }

    /// Runs the installer script while exposing only a metadata-provided target directory.
    fn run_with_metadata_target_no_env(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        metadata_target_dir: &Path,
    ) {
        let install_dir = self.home.join(".local/bin");
        let baseline_path = env::var("PATH").unwrap_or_default();
        let mut path_parts = vec![self.fake_bin.display().to_string()];

        if matches!(path_mode, PathMode::WithInstallDirInPath) {
            path_parts.push(install_dir.display().to_string());
        }
        if !baseline_path.is_empty() {
            path_parts.push(baseline_path);
        }

        let output = StdCommand::new("bash")
            .arg("install-locally.sh")
            .current_dir(&self.workspace)
            .env_remove("CARGO_TARGET_DIR")
            .env("HOME", &self.home)
            .env("SHELL", shell)
            .env("FAKE_BUILD_TAG", build_tag)
            .env("FAKE_METADATA_TARGET_DIR", metadata_target_dir)
            .env("PATH", path_parts.join(":"))
            .output()
            .expect("run installer");

        if !output.status.success() {
            panic!(
                "installer failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Runs the installer script with optional Cargo target directory override.
    fn run_with_optional_target_dir(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        target_dir: Option<&Path>,
    ) {
        let install_dir = self.home.join(".local/bin");
        let baseline_path = env::var("PATH").unwrap_or_default();
        let mut path_parts = vec![self.fake_bin.display().to_string()];

        if matches!(path_mode, PathMode::WithInstallDirInPath) {
            path_parts.push(install_dir.display().to_string());
        }
        if !baseline_path.is_empty() {
            path_parts.push(baseline_path);
        }

        let mut command = StdCommand::new("bash");
        command
            .arg("install-locally.sh")
            .current_dir(&self.workspace)
            .env("HOME", &self.home)
            .env("SHELL", shell)
            .env("FAKE_BUILD_TAG", build_tag)
            .env("PATH", path_parts.join(":"));

        let effective_target_dir = target_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| self.workspace.join("target"));
        command.env("CARGO_TARGET_DIR", &effective_target_dir);

        let output = command.output().expect("run installer");

        if !output.status.success() {
            panic!(
                "installer failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Returns synthetic HOME used by this fixture.
    fn home_dir(&self) -> &Path {
        &self.home
    }
}

/// Resolves repository root path from this crate's manifest directory.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above crate manifest")
        .to_path_buf()
}

/// Returns a fake `cargo` script that emits deterministic local build artifacts.
fn fake_cargo_script() -> &'static str {
    r#"#!/usr/bin/env sh
set -eu

if [ "$#" -lt 1 ] || [ "$1" != "build" ]; then
  if [ "$#" -ge 1 ] && [ "$1" = "metadata" ]; then
    target_dir="${CARGO_TARGET_DIR:-${FAKE_METADATA_TARGET_DIR:-target}}"
    printf '{"target_directory":"%s"}\n' "${target_dir}"
    exit 0
  fi

  printf 'unexpected cargo invocation: %s\n' "$*" >&2
  exit 1
fi

case " $* " in
  *" --release "* ) ;;
  * ) printf 'missing --release in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" --locked "* ) ;;
  * ) printf 'missing --locked in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" -p tak "* ) ;;
  * ) printf 'missing -p tak in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" -p takd "* ) ;;
  * ) printf 'missing -p takd in cargo invocation\n' >&2; exit 1 ;;
esac

target_dir="${CARGO_TARGET_DIR:-${FAKE_METADATA_TARGET_DIR:-target}}"
tag="${FAKE_BUILD_TAG:-dev}"

mkdir -p "${target_dir}/release"

cat > "${target_dir}/release/tak" <<EOF
#!/usr/bin/env sh
if [ "\${1:-}" = "--version" ]; then
  printf 'tak %s\n' "${tag}"
else
  printf 'tak %s\n' "${tag}"
fi
EOF

cat > "${target_dir}/release/takd" <<EOF
#!/usr/bin/env sh
printf 'takd %s\n' "${tag}"
EOF

chmod +x "${target_dir}/release/tak" "${target_dir}/release/takd"
"#
}
