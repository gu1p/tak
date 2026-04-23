use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output};

use super::fake_cargo::fake_cargo_script;

pub struct CoverageScriptFixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    fake_bin: PathBuf,
}

impl Default for CoverageScriptFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverageScriptFixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let fake_bin = temp.path().join("fake-bin");
        let scripts = workspace.join("scripts");

        fs::create_dir_all(&scripts).expect("create scripts dir");
        fs::create_dir_all(&fake_bin).expect("create fake-bin");

        let script_dst = scripts.join("run_coverage.sh");
        fs::copy(repo_root().join("scripts/run_coverage.sh"), &script_dst)
            .expect("copy run_coverage.sh");
        chmod_exec(&script_dst);

        let cargo_path = fake_bin.join("cargo");
        fs::write(&cargo_path, fake_cargo_script()).expect("write fake cargo");
        chmod_exec(&cargo_path);

        Self {
            _temp: temp,
            workspace,
            fake_bin,
        }
    }

    pub fn run(&self) -> Output {
        let mut path_parts = vec![self.fake_bin.display().to_string()];
        let baseline_path = env::var("PATH").unwrap_or_default();
        if !baseline_path.is_empty() {
            path_parts.push(baseline_path);
        }

        StdCommand::new("bash")
            .arg("scripts/run_coverage.sh")
            .current_dir(&self.workspace)
            .env("PATH", path_parts.join(":"))
            .output()
            .expect("run coverage script")
    }

    pub fn coverage_report_path(&self) -> PathBuf {
        self.workspace.join(".tmp/coverage/lcov.info")
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("repo root")
}

fn chmod_exec(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}
