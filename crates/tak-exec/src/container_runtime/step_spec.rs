use super::*;

pub(super) fn build_container_step_spec(
    step: &StepDef,
    workspace_root: &Path,
    base_environment: Option<&BTreeMap<String, String>>,
    runtime_env: Option<&BTreeMap<String, String>>,
    private_root: Option<&Path>,
) -> Result<ContainerStepSpec> {
    let (argv, cwd, step_env) = match step {
        StepDef::Cmd { argv, cwd, env } => {
            if argv.is_empty() {
                bail!("cmd step requires a non-empty argv");
            }
            (argv.clone(), cwd, env)
        }
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => {
            let mut full_argv = Vec::with_capacity(argv.len() + 2);
            if let Some(interpreter) = interpreter {
                full_argv.push(interpreter.clone());
            }
            full_argv.push(path.clone());
            full_argv.extend(argv.clone());
            (full_argv, cwd, env)
        }
    };
    let mut env = base_environment.cloned().unwrap_or_default();
    translate_private_environment(&mut env, private_root);
    if let Some(runtime_env) = runtime_env {
        env.extend(runtime_env.clone());
    }
    env.extend(step_env.clone());
    Ok(ContainerStepSpec {
        argv,
        cwd: resolve_cwd(workspace_root, cwd),
        env,
    })
}

fn translate_private_environment(
    environment: &mut BTreeMap<String, String>,
    private_root: Option<&Path>,
) {
    let Some(private_root) = private_root else {
        return;
    };
    let home = private_root.join("home").display().to_string();
    let temporary = private_root.join("tmp").display().to_string();
    if environment.get("HOME") == Some(&home) {
        environment.insert("HOME".into(), "/tmp/tak-home".into());
    }
    for name in ["TMPDIR", "TMP", "TEMP"] {
        if environment.get(name) == Some(&temporary) {
            environment.insert(name.into(), "/tmp/tak-tmp".into());
        }
    }
}
