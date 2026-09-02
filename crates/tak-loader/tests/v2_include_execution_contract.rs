use std::fs;

use tak_core::v2::{OutputSelector, Step, TaskRuntime};
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn included_task_paths_are_anchored_to_their_package() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let child = temp.path().join("apps/web");
    fs::create_dir_all(&child).unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC=module_spec(spec_version=2, includes=[path('apps/web')], tasks=[])\nSPEC\n",
    )
    .unwrap();
    fs::write(
        child.join("TASKS.py"),
        r#"SPEC=module_spec(spec_version=2, tasks=[task("build",
  steps=[cmd("sh", "-c", "true", cwd="src"), script("tool.sh")],
  outputs=[path("out/result.txt")])])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let task = &root.module.tasks[0];
    assert_eq!(task.name, "//apps/web:build");
    assert!(matches!(
        &task.steps[0],
        Step::Cmd { cwd: Some(path), .. } if path == "apps/web/src"
    ));
    assert!(matches!(
        &task.steps[1],
        Step::Script { path, .. } if path == "apps/web/tool.sh"
    ));
    assert_eq!(
        task.outputs,
        [OutputSelector::Path {
            value: "apps/web/out/result.txt".into()
        }]
    );
}

#[test]
fn included_container_mounts_keep_root_and_package_relative_sources_canonical() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("apps/web");
    fs::create_dir_all(&child).unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC=module_spec(spec_version=2, includes=[path('apps/web')], tasks=[])\nSPEC\n",
    )
    .unwrap();
    fs::write(
        child.join("TASKS.py"),
        r#"SPEC=module_spec(spec_version=2, tasks=[task("build",
  execution=Execution.Local(container=Container.Image("alpine:3.20", mounts=[
    {"source": "//root-cache", "target": "/root-cache", "read_only": True},
    {"source": "package-cache", "target": "/package-cache", "read_only": False},
  ])))])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let Some(TaskRuntime::Container { mounts, .. }) = root.module.tasks[0]
        .execution
        .as_ref()
        .and_then(tak_core::v2::Execution::runtime)
    else {
        panic!("included task must preserve its container runtime")
    };
    assert_eq!(
        mounts
            .iter()
            .map(|mount| mount.source.as_str())
            .collect::<Vec<_>>(),
        ["apps/web/package-cache", "root-cache"]
    );
}
