use std::fs;
use std::sync::{Mutex, OnceLock};
use tak_core::model::{ExecutionPlacementSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
struct EnvGuard(Option<String>);
impl EnvGuard {
    fn set_xdg_config_home(value: &std::path::Path) -> Self {
        let previous = std::env::var("XDG_CONFIG_HOME").ok();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", value) };
        Self(previous)
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => unsafe { std::env::set_var("XDG_CONFIG_HOME", value) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
    }
}
#[test]
fn global_config_default_execution_policy_applies_to_tasks_without_repo_default() {
    let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let _env = EnvGuard::set_xdg_config_home(&temp.path().join("config"));
    write_global_config(&temp.path().join("config"), "global");
    write_tasks(
        temp.path(),
        "SPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"true\")])])\nSPEC\n",
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    assert_policy_remote_pool(&spec, "global");
}
#[test]
fn repo_policy_definition_overrides_global_policy_with_same_name() {
    let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let _env = EnvGuard::set_xdg_config_home(&temp.path().join("config"));
    write_global_config(&temp.path().join("config"), "global");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
POLICY = execution_policy("default", [Execution.Remote(pool="repo", runtime=RUNTIME)])
SPEC = module_spec(execution_policies=[POLICY], tasks=[task("check", steps=[cmd("true")])])
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    assert_policy_remote_pool(&spec, "repo");
}

fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root).expect("create workspace");
    fs::write(root.join("TASKS.py"), body).expect("write TASKS.py");
}

fn write_global_config(config_root: &std::path::Path, pool: &str) {
    let path = config_root.join("tak").join("config.toml");
    fs::create_dir_all(path.parent().expect("config parent")).expect("create config parent");
    fs::write(
        path,
        format!(
            r#"[defaults]
execution_policy = "default"

[[execution_policies]]
name = "default"

[[execution_policies.placements]]
kind = "remote_only"
[execution_policies.placements.remote]
pool = "{pool}"
runtime = {{ kind = "containerized", image = "alpine:3.20" }}
"#
        ),
    )
    .expect("write config");
}

fn assert_policy_remote_pool(spec: &tak_core::model::WorkspaceSpec, expected_pool: &str) {
    let task = spec.tasks.values().next().expect("task");
    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => {
            assert_eq!(name, "default");
            match placements.first().expect("placement") {
                ExecutionPlacementSpec::Remote(remote) => {
                    assert_eq!(remote.pool.as_deref(), Some(expected_pool));
                }
                other => panic!("expected remote placement, got {other:?}"),
            }
        }
        other => panic!("expected global execution policy, got {other:?}"),
    }
}
