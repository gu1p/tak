use super::*;
use serde_json::json;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn runtime_env_safe_mounts_and_private_home_follow_precedence() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().unwrap();
    let exec_root = temp.path().join("exec");
    let docker = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), docker.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .build();
    let server = start_server_with_runtime(runtime).await;
    dispatch(&server, configured_request()).await;

    let creates = docker.create_records();
    let create = creates.first().expect("one container");
    for expected in ["ORDER=step", "RUNTIME_WINS=runtime", "RUNTIME_ONLY=set"] {
        assert!(create.env.contains(&expected.to_string()), "{create:?}");
    }
    for expected in [
        "HOME=/tmp/tak-home",
        "TMPDIR=/tmp/tak-tmp",
        "TMP=/tmp/tak-tmp",
        "TEMP=/tmp/tak-tmp",
    ] {
        assert!(create.env.contains(&expected.to_string()), "{create:?}");
    }
    for suffix in [
        ":/inputs/workspace:ro",
        ":/tmp/tak-home:rw",
        ":/tmp/tak-tmp:rw",
    ] {
        assert!(
            create.binds.iter().any(|bind| bind.ends_with(suffix)),
            "{create:?}"
        );
    }
    #[cfg(unix)]
    for target in ["/tmp/tak-home", "/tmp/tak-tmp"] {
        assert_eq!(
            create.bind_modes.get(target),
            Some(&0o777),
            "container-private {target} must be writable by the image user: {create:?}"
        );
    }
}

fn configured_request() -> DispatchAttemptRequest {
    let mut request = output_dispatch();
    request.identity.run_id = "run-configured".into();
    request.identity.job_id = "job-configured".into();
    request.identity.fencing_token = "fence-configured".into();
    let task = &mut request.payload.tasks[0];
    task.job_id = request.identity.job_id.clone();
    task.runtime = Some(
        serde_json::from_value(json!({
            "kind": "container",
            "source": {"kind": "image", "image": "alpine:3.20"},
            "mounts": [{"source": ".", "target": "/inputs/workspace", "read_only": true}],
            "env": {"ORDER": "runtime", "RUNTIME_WINS": "runtime", "RUNTIME_ONLY": "set"},
        }))
        .unwrap(),
    );
    task.steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "true".into()],
        cwd: None,
        env: BTreeMap::from([("ORDER".into(), "step".into())]),
    }];
    task.outputs.clear();
    task.pass_env_names = vec!["ORDER".into(), "RUNTIME_WINS".into()];
    request.payload.environment_values = vec![
        EnvironmentValue::new("ORDER", "passed").unwrap(),
        EnvironmentValue::new("RUNTIME_WINS", "passed").unwrap(),
    ];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}
