use tak_core::model::{RemoteRuntimeSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

use super::super::support::write_root_and_app_tasks;

#[test]
fn resolves_remote_runtime_with_typed_container_resources() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  container=Container.Image(
    "alpine:3.20",
    resources=Container.Resources(cpu_cores=2.5, memory_mb=1536),
  ),
)
SPEC = module_spec(tasks=[task("remote_only", steps=[cmd("echo", "ok")], execution=REMOTE)])
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec
        .tasks
        .values()
        .next()
        .expect("resolved task should exist");
    match &task.execution {
        TaskExecutionSpec::RemoteOnly(remote) => {
            let RemoteRuntimeSpec::Containerized {
                resource_limits, ..
            } = remote.runtime.as_ref().expect("remote runtime");
            let limits = resource_limits.as_ref().expect("resource limits");
            assert_eq!(limits.cpu_cores, Some(2.5));
            assert_eq!(limits.memory_mb, Some(1536));
        }
        other => panic!("expected RemoteOnly execution, got {other:?}"),
    }
}

#[test]
fn rejects_remote_runtime_without_complete_resources() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  container=Container.Image("alpine:3.20"),
)
SPEC = module_spec(tasks=[task("remote_only", steps=[cmd("echo", "ok")], execution=REMOTE)])
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("remote resources should be required");
    assert!(
        err.to_string()
            .contains("remote container resources require cpu_cores and memory_mb"),
        "{err:#}"
    );
}

#[test]
fn rejects_loose_dict_container_resources() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  container=Container.Image("alpine:3.20", resources={"cpu_cores": 1.0, "memory_mb": 512}),
)
SPEC = module_spec(tasks=[task("remote_only", steps=[cmd("echo", "ok")], execution=REMOTE)])
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("loose dict resources should be rejected");
    assert!(
        err.to_string()
            .contains("resources must be created with Container.Resources(...)"),
        "{err:#}"
    );
}
