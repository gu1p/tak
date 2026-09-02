use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use rusqlite::Connection;

use super::{Installer, archive, fake_tools};

pub(super) struct Run {
    pub(super) _temp: tempfile::TempDir,
    pub(super) install_dir: PathBuf,
    pub(super) output: Output,
}

pub(super) fn run(installer: Installer, active: bool) -> Run {
    let temp = tempfile::tempdir().expect("installer tempdir");
    let root = temp.path();
    let home = root.join("home");
    let bin = root.join("bin");
    let workspace = root.join("workspace");
    let install_dir = home.join(".local/bin");
    let state_home = home.join(".local/state");
    let state_root = state_home.join("takd");
    let target = workspace.join("target");
    for directory in [&home, &bin, &workspace, &install_dir] {
        fs::create_dir_all(directory).expect("create fixture directory");
    }
    fs::copy(
        repo_root().join(installer.script()),
        workspace.join(installer.script()),
    )
    .expect("copy installer");
    fake_tools::install(&bin);
    let release_archive = archive::create(root);
    archive::install_build_artifacts(&target);
    seed_sentinels(&install_dir);
    if active {
        seed_active_store(&state_root);
    }
    let output = Command::new("bash")
        .arg(installer.script())
        .current_dir(&workspace)
        .env("HOME", &home)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_STATE_HOME", &state_home)
        .env("TAK_INSTALL_DIR", &install_dir)
        .env("TAKD_INSTALL_DIR", &install_dir)
        .env("TAK_VERSION", "1.2.3")
        .env("CARGO_TARGET_DIR", &target)
        .env("FAKE_RELEASE_ARCHIVE", &release_archive)
        .output()
        .expect("run installer");
    Run {
        _temp: temp,
        install_dir,
        output,
    }
}

fn seed_sentinels(install_dir: &std::path::Path) {
    fs::write(install_dir.join("tak"), "old-tak\n").expect("seed tak");
    fs::write(install_dir.join("takd"), "old-takd\n").expect("seed takd");
}

fn seed_active_store(state_root: &std::path::Path) {
    fs::create_dir_all(state_root).expect("create state root");
    let connection = Connection::open(state_root.join("agent.sqlite")).expect("legacy db");
    connection
        .execute_batch("CREATE TABLE submit_attempts (idempotency_key TEXT PRIMARY KEY); INSERT INTO submit_attempts VALUES ('active');")
        .expect("seed active legacy attempt");
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
