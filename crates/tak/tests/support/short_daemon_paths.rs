#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub struct DaemonCommandPaths<'a> {
    command_root: &'a Path,
    config_root: &'a Path,
    state_root: &'a Path,
}

impl<'a> DaemonCommandPaths<'a> {
    pub fn new(config_root: &'a Path, state_root: &'a Path) -> Self {
        let command_root = state_root
            .parent()
            .expect("daemon state root should have a parent");
        assert_eq!(
            config_root.parent(),
            Some(command_root),
            "daemon config and state roots should share one temp root"
        );
        Self {
            command_root,
            config_root: config_root
                .strip_prefix(command_root)
                .expect("config root should be below command root"),
            state_root: state_root
                .strip_prefix(command_root)
                .expect("state root should be below command root"),
        }
    }

    pub fn command_root(&self) -> &Path {
        self.command_root
    }

    pub fn config_root(&self) -> &Path {
        self.config_root
    }

    pub fn state_root(&self) -> &Path {
        self.state_root
    }

    pub fn remote_exec_root(&self) -> &Path {
        Path::new("remote-exec")
    }

    pub fn runtime_root(&self) -> &Path {
        Path::new("runtime")
    }

    pub fn rooted_command(&self, executable: &Path, subcommand: &str) -> Command {
        let mut command = self.command(executable);
        command
            .arg(subcommand)
            .arg("--config-root")
            .arg(self.config_root)
            .arg("--state-root")
            .arg(self.state_root);
        command
    }

    pub fn state_command(&self, executable: &Path, subcommands: &[&str]) -> Command {
        let mut command = self.command(executable);
        command
            .args(subcommands)
            .arg("--state-root")
            .arg(self.state_root);
        command
    }

    fn command(&self, executable: &Path) -> Command {
        let mut command = Command::new(executable);
        command.current_dir(self.command_root);
        command
    }
}
