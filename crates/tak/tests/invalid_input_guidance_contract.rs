mod support;

use anyhow::Result;

use support::{run_tak_expect_failure, write_tasks};

#[test]
fn run_dot_fails_with_guidance_and_discovered_targets() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        r#"
SPEC = module_spec(tasks=[
  task("hello", steps=[cmd("sh", "-c", "mkdir -p out && echo hello > out/hello.txt")]),
  task("test", deps=[":hello"], steps=[cmd("echo", "ok")]),
])
SPEC
"#,
    )?;

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "."], &Default::default())?;
    assert!(
        stderr.contains("`.` is not a valid task label"),
        "stderr should explain invalid dot shorthand:\n{stderr}"
    );
    assert!(stderr.contains("tak list"), "stderr:\n{stderr}");
    assert!(stderr.contains("//:hello"), "stderr:\n{stderr}");
    assert!(stderr.contains("//:test"), "stderr:\n{stderr}");

    Ok(())
}
