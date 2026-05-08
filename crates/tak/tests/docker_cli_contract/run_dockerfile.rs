use std::fs;

use anyhow::Result;

use crate::support::{self, run_tak_output};

#[test]
fn docker_run_dockerfile_mode_treats_positionals_as_command() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("Dockerfile"), "FROM alpine:3.20\n")?;
    let mut env = support::container_runtime::simulated_container_runtime_env(temp.path());
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );

    let output = run_tak_output(
        temp.path(),
        &[
            "--local",
            "docker",
            "run",
            "-f",
            "Dockerfile",
            "--build-context",
            ".",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "dockerfile");
    Ok(())
}
