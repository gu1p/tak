use std::path::Path;
use std::process::{Command, Output};

pub fn run_takd_tasks(config_root: &Path, state_root: &Path) -> Output {
    let command_root = state_root.parent().expect("state root parent");
    let config_arg = config_root
        .strip_prefix(command_root)
        .expect("config and state roots must share a parent");
    let state_arg = state_root
        .strip_prefix(command_root)
        .expect("state root must be below its parent");
    Command::new(super::super::takd_bin())
        .args(["tasks", "--config-root"])
        .arg(config_arg)
        .arg("--state-root")
        .arg(state_arg)
        .current_dir(command_root)
        .output()
        .expect("run takd tasks")
}
