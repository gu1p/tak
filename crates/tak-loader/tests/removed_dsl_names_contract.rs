use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn removed_top_level_dsl_names_are_undefined_not_rewritten() {
    for (source, removed_name) in [
        (
            r#"SPEC = module_spec(tasks=[task("x", execution=LocalOnly(Local("x")))])
SPEC
"#,
            "LocalOnly",
        ),
        (
            r#"SPEC = module_spec(defaults={"container_runtime": ContainerRuntime("alpine:3.20")}, tasks=[])
SPEC
"#,
            "ContainerRuntime",
        ),
        (
            r#"SPEC = module_spec(tasks=[
  task("x", execution=Execution.Remote(transport=DirectHttps()))
])
SPEC
"#,
            "DirectHttps",
        ),
        (
            r#"SPEC = module_spec(limiters=[lock("ui", scope=MACHINE)], tasks=[])
SPEC
"#,
            "MACHINE",
        ),
        (
            r#"SPEC = module_spec(sessions=[
  session("x", execution=Execution.Local(), reuse=ShareWorkspace())
], tasks=[])
SPEC
"#,
            "ShareWorkspace",
        ),
        (
            r#"def choose(ctx):
  return Decision.local(reason=REASON_LOCAL_CPU_HIGH)

SPEC = module_spec(tasks=[task("x", execution=Execution.Policy(choose))])
SPEC
"#,
            "REASON_LOCAL_CPU_HIGH",
        ),
        (
            r#"SPEC = module_spec(tasks=[
  task("x", execution=Execution.Remote(transport=RemoteTransportMode.TorOnionService()))
])
SPEC
"#,
            "RemoteTransportMode",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");
        let err = load_workspace(temp.path(), &LoadOptions::default())
            .expect_err("removed DSL name should be undefined");
        let message = err.to_string();
        assert!(
            message.contains("unresolved-reference"),
            "removed name should fail as undefined, not through a compatibility path:\n{message:#}"
        );
        assert!(
            message.contains(removed_name),
            "missing removed name `{removed_name}` in error:\n{message:#}"
        );
    }
}
