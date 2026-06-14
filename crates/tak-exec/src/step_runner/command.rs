//! Translates a `StepDef` into a runnable process command and working directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tak_core::model::StepDef;
use tokio::process::Command;

/// Builds an executable process command and effective working directory for a step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn build_command(
    step: &StepDef,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<(Command, PathBuf)> {
    match step {
        StepDef::Cmd { argv, cwd, env } => {
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| anyhow!("cmd step requires a non-empty argv"))?;
            let mut command = Command::new(program);
            command.args(args);
            if let Some(runtime_env) = runtime_env {
                for (key, value) in runtime_env {
                    command.env(key, value);
                }
            }
            for (key, value) in env {
                command.env(key, value);
            }
            Ok((command, resolve_cwd(workspace_root, cwd)))
        }
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => {
            let mut command = if let Some(interpreter) = interpreter {
                let mut cmd = Command::new(interpreter);
                cmd.arg(path);
                cmd.args(argv);
                cmd
            } else {
                let mut cmd = Command::new(path);
                cmd.args(argv);
                cmd
            };
            if let Some(runtime_env) = runtime_env {
                for (key, value) in runtime_env {
                    command.env(key, value);
                }
            }
            for (key, value) in env {
                command.env(key, value);
            }
            Ok((command, resolve_cwd(workspace_root, cwd)))
        }
    }
}

/// Resolves a step-local working directory against the workspace root.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn resolve_cwd(workspace_root: &Path, cwd: &Option<String>) -> PathBuf {
    match cwd {
        Some(value) => {
            let path = Path::new(value);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            }
        }
        None => workspace_root.to_path_buf(),
    }
}

#[path = "command_tests.rs"]
mod command_tests;
