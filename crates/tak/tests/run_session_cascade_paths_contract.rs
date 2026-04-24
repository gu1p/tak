use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn parent_use_session_cascades_share_paths_to_dependency_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = ContainerRuntime(image="alpine:3.20")
SESSION = session(
  "cargo",
  execution=LocalOnly(Local("local", runtime=RUNTIME)),
  reuse=SharePaths([path("target")]),
)

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("compile", steps=[cmd("sh", "-c", "mkdir -p target scratch && echo cached > target/cache.txt && echo leak > scratch/leak.txt")]),
    task("check-artifact", deps=[":compile"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f target/cache.txt && test ! -e scratch/leak.txt && mkdir -p out && cat target/cache.txt > out/cache.txt")]),
    task("check", deps=[":check-artifact"], execution=UseSession("cargo", cascade=True)),
  ],
)
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("session=cargo"), "stdout:\n{stdout}");
    assert!(stdout.contains("reuse=share_paths"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/cache.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join("target/cache.txt").exists());
    assert!(!workspace.join("scratch/leak.txt").exists());
    Ok(())
}
