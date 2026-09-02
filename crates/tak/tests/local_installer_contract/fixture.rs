use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// PATH mode to simulate whether install dir is already discoverable in the running shell.
pub(super) enum PathMode {
    WithInstallDirInPath,
    WithoutInstallDirInPath,
}

/// Isolated test fixture for invoking `install-locally.sh` with a fake cargo build.
pub(super) struct InstallerFixture {
    pub(super) _temp: tempfile::TempDir,
    pub(super) workspace: PathBuf,
    pub(super) fake_bin: PathBuf,
    pub(super) home: PathBuf,
}

impl InstallerFixture {
    /// Creates an isolated workspace containing the installer script and a fake cargo binary.
    pub(super) fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let fake_bin = temp.path().join("fake-bin");
        let home = temp.path().join("home");

        fs::create_dir_all(&workspace).expect("create workspace");
        fs::create_dir_all(&fake_bin).expect("create fake-bin");
        fs::create_dir_all(&home).expect("create home");

        copy_installer(&workspace);
        create_fake_cargo(&fake_bin);

        Self {
            _temp: temp,
            workspace,
            fake_bin,
            home,
        }
    }

    /// Returns synthetic HOME used by this fixture.
    pub(super) fn home_dir(&self) -> &Path {
        &self.home
    }
}

fn copy_installer(workspace: &Path) {
    let script_src = repo_root().join("install-locally.sh");
    let script_dst = workspace.join("install-locally.sh");
    fs::copy(&script_src, &script_dst).unwrap_or_else(|err| {
        panic!(
            "failed to copy installer script {} -> {}: {err}",
            script_src.display(),
            script_dst.display()
        )
    });
}

fn create_fake_cargo(fake_bin: &Path) {
    let cargo_path = fake_bin.join("cargo");
    fs::write(&cargo_path, include_str!("fake_cargo.sh")).expect("write fake cargo");
    let mut perms = fs::metadata(&cargo_path)
        .expect("cargo metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&cargo_path, perms).expect("chmod fake cargo");
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above crate manifest")
        .to_path_buf()
}
