use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn local_session_workspace_prefers_workspace_tmp_directory() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("rust", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("capture", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && pwd > out/session-root.txt")], execution=Execution.Session("rust")),
  ],
)
SPEC
"#,
    )?;
    fs::create_dir_all(workspace.join(".tmp"))?;

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    run_tak_expect_success(&workspace, &["run", "capture"], &env)?;

    let session_root = fs::read_to_string(workspace.join("out/session-root.txt"))?;
    assert!(
        session_root.contains("/.tmp/tak-sessions/"),
        "session root should stay under workspace .tmp: {session_root}"
    );
    Ok(())
}
