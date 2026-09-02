use clap::Parser;
use sha2::{Digest, Sha256};
use tak_core::v2::{ContainerSource, TaskRuntime, WorkspaceDescriptor, WorkspaceManifest};

use super::command_model::{Cli, Commands};
use super::exec_cli::{ExecCliArgs, resolve};

#[test]
fn exec_parser_collects_repeatable_pass_env_names() {
    let cli = Cli::try_parse_from([
        "tak",
        "exec",
        "--pass-env",
        "ALPHA",
        "--pass-env",
        "BETA",
        "--",
        "true",
    ])
    .unwrap();

    let Commands::Exec { pass_env, .. } = cli.command else {
        panic!("expected exec command")
    };
    assert_eq!(pass_env, ["ALPHA", "BETA"]);
}

#[tokio::test]
async fn exec_submission_carries_v2_runtime_and_step_overrides() {
    let args = ExecCliArgs {
        cwd: Some("work".into()),
        env: vec!["INLINE=value".into()],
        pass_env: Vec::new(),
        local: true,
        local_no_container: false,
        remote: false,
        container: true,
        container_image: Some("alpine:3.20".into()),
        container_dockerfile: None,
        container_build_context: None,
        argv: vec!["true".into()],
    };
    let submission = resolve::submission(&args, empty_workspace()).await.unwrap();
    let task = &submission.run.tasks[0];

    assert_eq!(task.timeout_s, None);
    assert_eq!(
        task.runtime,
        Some(TaskRuntime::container(ContainerSource::Image {
            image: "alpine:3.20".into(),
        }))
    );
    assert_eq!(
        task.steps,
        [tak_core::v2::Step::Cmd {
            argv: vec!["true".into()],
            cwd: Some("work".into()),
            env: [("INLINE".into(), "value".into())].into(),
        }]
    );
}

fn empty_workspace() -> WorkspaceDescriptor {
    WorkspaceDescriptor {
        manifest: WorkspaceManifest::new([]).unwrap(),
        archive_sha256: format!("{:x}", Sha256::digest([])),
        archive_size: 0,
    }
}
