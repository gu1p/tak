//! Black-box contract for `tak exec` local developer workflows.

mod support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;
use support::container_runtime::simulated_container_runtime_env;
use support::run_tak_output;

#[test]
fn exec_runs_raw_command_without_needing_tasks_py() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let env = BTreeMap::new();
    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--",
            "sh",
            "-c",
            "printf 'stdout-line\\n'; printf 'stderr-line\\n' >&2; exit 7",
        ],
        &env,
    )?;

    assert_eq!(output.status.code(), Some(7), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "stdout-line\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "stderr-line\n");
    Ok(())
}

#[test]
fn exec_honors_cwd_and_env_overrides() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workdir = temp.path().join("work");
    fs::create_dir_all(&workdir)?;

    let env = BTreeMap::new();
    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--cwd",
            "work",
            "--env",
            "HELLO=world",
            "--",
            "sh",
            "-c",
            "printf '%s\\n%s\\n' \"$PWD\" \"$HELLO\"",
        ],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some(workdir.display().to_string().as_str()));
    assert_eq!(lines.next(), Some("world"));
    assert_eq!(lines.next(), None);
    Ok(())
}

#[test]
fn exec_supports_local_container_runtime_override() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(
        temp.path().join("docker/Dockerfile"),
        "FROM alpine:3.20\nRUN printf 'built\\n' > /tmp/built.txt\n",
    )?;

    let mut env = BTreeMap::new();
    env.extend(simulated_container_runtime_env(temp.path()));

    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--local",
            "--container",
            "--container-dockerfile",
            "docker/Dockerfile",
            "--",
            "sh",
            "-c",
            "mkdir -p out && printf '%s\\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt",
        ],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        fs::read_to_string(temp.path().join("out/runtime-source.txt"))?.trim(),
        "dockerfile"
    );
    Ok(())
}
