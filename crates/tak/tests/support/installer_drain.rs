#[path = "installer_drain/archive.rs"]
mod archive;
#[path = "installer_drain/fake_tools.rs"]
mod fake_tools;
#[path = "installer_drain/runner.rs"]
mod runner;

use std::fs;

#[derive(Clone, Copy)]
pub enum Installer {
    ReleaseBundle,
    Agent,
    Source,
}

impl Installer {
    pub(super) fn script(self) -> &'static str {
        match self {
            Self::ReleaseBundle => "get-tak.sh",
            Self::Agent => "get-takd.sh",
            Self::Source => "install-locally.sh",
        }
    }

    pub(super) fn installs_tak(self) -> bool {
        !matches!(self, Self::Agent)
    }
}

pub fn assert_drain_contract(installer: Installer) {
    let blocked = runner::run(installer, true);
    let stderr = String::from_utf8_lossy(&blocked.output.stderr);
    assert!(
        !blocked.output.status.success(),
        "active legacy work must block replacement"
    );
    assert!(
        stderr.contains("active legacy attempts must finish"),
        "stderr: {stderr}"
    );
    assert_eq!(read(&blocked, "takd"), "old-takd\n");
    if installer.installs_tak() {
        assert_eq!(read(&blocked, "tak"), "old-tak\n");
    }

    let idle = runner::run(installer, false);
    assert!(
        idle.output.status.success(),
        "idle install failed: {:?}",
        idle.output
    );
    assert!(read(&idle, "takd").contains("new-takd"));
    if installer.installs_tak() {
        assert!(read(&idle, "tak").contains("new-tak"));
    }
}

fn read(run: &runner::Run, binary: &str) -> String {
    fs::read_to_string(run.install_dir.join(binary)).expect("read installed binary")
}
