//! Contract tests for release packaging scripts.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use anyhow::Result;

#[test]
fn package_script_rejects_takd_version_mismatch() -> Result<()> {
    let fixture = PackageFixture::new()?;
    fixture.write_binary("tak", "1.2.3")?;
    fixture.write_binary("takd", "9.9.9")?;

    let output = StdCommand::new("bash")
        .arg("scripts/package_release_target.sh")
        .arg(fixture.target())
        .current_dir(repo_root())
        .env("TAK_RELEASE_TAG", "v1.2.3")
        .env("TAK_BUILD_VERSION", "1.2.3")
        .env("TAK_RELEASE_TARGET_ROOT", fixture.target_root())
        .env("TAK_DIST_ROOT", fixture.dist_root())
        .output()?;

    assert!(
        !output.status.success(),
        "package should fail when takd reports the wrong version"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("version mismatch for takd"),
        "unexpected stderr:\n{stderr}"
    );
    Ok(())
}

struct PackageFixture {
    _temp: tempfile::TempDir,
    target: &'static str,
    target_root: PathBuf,
    dist_root: PathBuf,
}

impl PackageFixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        Ok(Self {
            target: "test-target",
            target_root: temp.path().join("release-target"),
            dist_root: temp.path().join("dist"),
            _temp: temp,
        })
    }

    fn write_binary(&self, name: &str, version: &str) -> Result<()> {
        let release_dir = self
            .target_root
            .join(self.target)
            .join(self.target)
            .join("release");
        fs::create_dir_all(&release_dir)?;
        let binary = release_dir.join(name);
        fs::write(
            &binary,
            format!("#!/usr/bin/env sh\nprintf '{name} {version}\\n'\n"),
        )?;
        let mut permissions = fs::metadata(&binary)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(binary, permissions)?;
        Ok(())
    }

    fn target(&self) -> &str {
        self.target
    }

    fn target_root(&self) -> &Path {
        &self.target_root
    }

    fn dist_root(&self) -> &Path {
        &self.dist_root
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
