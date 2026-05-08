use anyhow::{Result, bail};

#[derive(Debug, Default)]
pub(in crate::cli::docker_cli) struct DockerRunSpec {
    pub(in crate::cli::docker_cli) image: Option<String>,
    pub(in crate::cli::docker_cli) dockerfile: Option<String>,
    pub(in crate::cli::docker_cli) build_context: Option<String>,
    pub(in crate::cli::docker_cli) argv: Vec<String>,
    pub(in crate::cli::docker_cli) publishes: Vec<String>,
    pub(in crate::cli::docker_cli) volumes: Vec<String>,
    pub(in crate::cli::docker_cli) env: Vec<String>,
    pub(in crate::cli::docker_cli) workdir: Option<String>,
    pub(in crate::cli::docker_cli) name: Option<String>,
    pub(in crate::cli::docker_cli) cpus: Option<String>,
    pub(in crate::cli::docker_cli) memory: Option<String>,
    pub(in crate::cli::docker_cli) rm: bool,
}

pub(in crate::cli::docker_cli) fn parse_docker_run(args: &[String]) -> Result<DockerRunSpec> {
    let mut spec = DockerRunSpec::default();
    let mut index = 0_usize;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            spec.argv.extend(args[index + 1..].iter().cloned());
            return Ok(spec);
        }
        if arg == "-d" || arg == "--detach" {
            bail!("tak docker run does not support detached containers");
        }
        if arg == "--rm" {
            spec.rm = true;
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--publish") {
            spec.publishes.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-p" || arg == "--publish" {
            spec.publishes.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-p") {
            spec.publishes.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--volume") {
            spec.volumes.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-v" || arg == "--volume" {
            spec.volumes.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-v") {
            spec.volumes.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--env") {
            spec.env.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-e" || arg == "--env" {
            spec.env.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-e") {
            spec.env.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--workdir") {
            spec.workdir = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-w" || arg == "--workdir" {
            spec.workdir = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-w") {
            spec.workdir = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--name") {
            spec.name = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--name" {
            spec.name = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = long_value(arg, "--cpus") {
            spec.cpus = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--cpus" {
            spec.cpus = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = long_value(arg, "--memory") {
            spec.memory = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-m" || arg == "--memory" {
            spec.memory = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-m") {
            spec.memory = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--file") {
            spec.dockerfile = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-f" || arg == "--file" {
            spec.dockerfile = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-f") {
            spec.dockerfile = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--build-context") {
            spec.build_context = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--build-context" {
            spec.build_context = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if arg.starts_with('-') {
            bail!("tak docker run does not support Docker flag `{arg}` yet");
        }

        if spec.dockerfile.is_some() {
            spec.argv.extend(args[index..].iter().cloned());
        } else {
            spec.image = Some(arg.clone());
            spec.argv.extend(args[index + 1..].iter().cloned());
        }
        return Ok(spec);
    }
    Ok(spec)
}

fn long_value<'a>(arg: &'a str, flag: &str) -> Option<&'a str> {
    arg.strip_prefix(flag)
        .and_then(|value| value.strip_prefix('='))
        .filter(|value| !value.is_empty())
}

fn short_attached_value<'a>(arg: &'a str, flag: &str) -> Option<&'a str> {
    arg.strip_prefix(flag).filter(|value| !value.is_empty())
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    let next = *index + 1;
    let Some(value) = args.get(next) else {
        bail!("{flag} requires a value");
    };
    *index += 2;
    Ok(value.clone())
}
