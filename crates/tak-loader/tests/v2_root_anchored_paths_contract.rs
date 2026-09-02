use std::fs;

use tak_core::v2::{OutputSelector, Step};
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn included_tasks_can_explicitly_address_the_workspace_root() {
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
  steps=[cmd("sh", "-c", "true", cwd="//")],
  outputs=[path("//out/result.txt")],
  context=CurrentState(roots=[path("//src")]))])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let task = &root.module.tasks[0];
    assert!(matches!(&task.steps[0], Step::Cmd { cwd: Some(path), .. } if path == "."));
    assert_eq!(
        task.outputs,
        [OutputSelector::Path {
            value: "out/result.txt".into()
        }]
    );
    assert_eq!(task.context.as_ref().unwrap().roots, ["src"]);
}
