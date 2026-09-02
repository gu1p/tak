use std::env;
use std::path::Path;
use std::process::{Command, Output};

use super::fixture::{InstallerFixture, PathMode};

impl InstallerFixture {
    pub(super) fn run(&self, build_tag: &str, shell: &str, path_mode: PathMode) {
        let target_dir = self.workspace.join("target");
        self.execute(build_tag, shell, path_mode, Some(&target_dir), None);
    }

    pub(super) fn run_with_target_dir(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        target_dir: &Path,
    ) {
        self.execute(build_tag, shell, path_mode, Some(target_dir), None);
    }

    pub(super) fn run_with_metadata_target_no_env(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        metadata_target_dir: &Path,
    ) {
        self.execute(build_tag, shell, path_mode, None, Some(metadata_target_dir));
    }

    fn execute(
        &self,
        build_tag: &str,
        shell: &str,
        path_mode: PathMode,
        target_dir: Option<&Path>,
        metadata_target_dir: Option<&Path>,
    ) {
        let mut command = self.command(build_tag, shell, path_mode);
        match target_dir {
            Some(path) => {
                command.env("CARGO_TARGET_DIR", path);
            }
            None => {
                command.env_remove("CARGO_TARGET_DIR");
            }
        }
        if let Some(path) = metadata_target_dir {
            command.env("FAKE_METADATA_TARGET_DIR", path);
        }
        assert_success(command.output().expect("run installer"));
    }

    fn command(&self, build_tag: &str, shell: &str, path_mode: PathMode) -> Command {
        let mut command = Command::new("bash");
        command
            .arg("install-locally.sh")
            .current_dir(&self.workspace)
            .env("HOME", &self.home)
            .env("SHELL", shell)
            .env("FAKE_BUILD_TAG", build_tag)
            .env("PATH", self.path(path_mode));
        command
    }

    fn path(&self, path_mode: PathMode) -> String {
        let mut parts = vec![self.fake_bin.display().to_string()];
        if matches!(path_mode, PathMode::WithInstallDirInPath) {
            parts.push(self.home.join(".local/bin").display().to_string());
        }
        let baseline = env::var("PATH").unwrap_or_default();
        if !baseline.is_empty() {
            parts.push(baseline);
        }
        parts.join(":")
    }
}

fn assert_success(output: Output) {
    if !output.status.success() {
        panic!(
            "installer failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
