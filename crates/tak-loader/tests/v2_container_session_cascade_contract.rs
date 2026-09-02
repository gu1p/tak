use std::fs;

use tak_core::v2::SessionReuse;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn v2_loader_preserves_container_session_cascade() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"S = session("build", execution=Execution.Local(),
  reuse=SessionReuse.Container())
SPEC = module_spec(spec_version=2, tasks=[
  task("dep"),
  task("target", deps=[":dep"], use_session=S, cascade_session=True),
])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let dependency = root
        .module
        .tasks
        .iter()
        .find(|task| task.name == "//:dep")
        .unwrap();
    let target = root
        .module
        .tasks
        .iter()
        .find(|task| task.name == "//:target")
        .unwrap();
    assert!(dependency.session.is_none());
    assert!(target.cascade_session);
    assert!(matches!(
        target.session.as_ref().unwrap().reuse,
        SessionReuse::Container
    ));
}

#[test]
fn cascade_requires_an_explicit_session_source() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(spec_version=2, tasks=[task('check', cascade_session=True)])\nSPEC\n",
    )
    .unwrap();
    let error = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("cascade_session=True) requires use_session"),
        "{error}"
    );
}
