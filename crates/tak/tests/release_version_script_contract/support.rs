use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output};

use anyhow::Result;

pub struct GitFixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
}

impl GitFixture {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo)?;
        run_git(&repo, &["init"])?;
        run_git(&repo, &["config", "user.name", "Tak Tests"])?;
        run_git(
            &repo,
            &["config", "user.email", "tak-tests@example.invalid"],
        )?;

        let fixture = Self { _temp: temp, repo };
        fixture.commit("initial")?;
        Ok(fixture)
    }

    pub fn commit(&self, name: &str) -> Result<()> {
        fs::write(self.repo.join("file.txt"), name)?;
        run_git(&self.repo, &["add", "file.txt"])?;
        run_git(&self.repo, &["commit", "-m", name])?;
        Ok(())
    }

    pub fn tag(&self, tag: &str) -> Result<()> {
        run_git(&self.repo, &["tag", tag])?;
        Ok(())
    }

    pub fn compute(&self, workspace_version: &str) -> Result<String> {
        let output = StdCommand::new("bash")
            .arg(repo_root().join("scripts/compute_release_version.sh"))
            .arg(workspace_version)
            .arg(self.head_sha()?)
            .current_dir(&self.repo)
            .output()?;
        assert_success(output)
    }

    fn head_sha(&self) -> Result<String> {
        let output = run_git(&self.repo, &["rev-parse", "HEAD"])?;
        Ok(output.trim().to_string())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(repo)
        .output()?;
    assert_success(output)
}

fn assert_success(output: Output) -> Result<String> {
    assert!(
        output.status.success(),
        "command failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}
