use super::*;
use serde_json::json;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn runtime_env_safe_mounts_and_private_home_follow_precedence() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().unwrap();
    let docker = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![temp.path().to_path_buf()],
            ..Default::default()
        },
    );
    let _runtime = configure_fake_docker_env(temp.path(), docker.socket_path(), &mut env).build();
    let db = temp.path().join("takd.sqlite");
    let socket = temp.path().join("takd.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    assert!(wait_for(|| socket.exists()).await);
    let store = RunStore::with_db_path(db).unwrap();
    commit_and_wait(&store, configured_run()).await;

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
        ":/inputs/TASKS.py:ro",
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
    server.abort();
}

fn configured_run() -> RunSubmission {
    let mut request = v2_run::submission("configured-container", "unused");
    let task = &mut request.run.tasks[0];
    task.runtime = Some(
        serde_json::from_value(json!({
            "kind": "container",
            "source": {"kind": "image", "image": "alpine:3.20"},
            "mounts": [{"source": "TASKS.py", "target": "/inputs/TASKS.py", "read_only": true}],
            "env": {"ORDER": "runtime", "RUNTIME_WINS": "runtime", "RUNTIME_ONLY": "set"},
        }))
        .unwrap(),
    );
    task.steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "true".into()],
        cwd: None,
        env: BTreeMap::from([("ORDER".into(), "step".into())]),
    }];
    task.pass_env_names = vec!["ORDER".into(), "RUNTIME_WINS".into()];
    request.run.jobs[0].pass_env_names = task.pass_env_names.clone();
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        vec![
            EnvironmentValue::new("ORDER", "passed").unwrap(),
            EnvironmentValue::new("RUNTIME_WINS", "passed").unwrap(),
        ],
    )
    .unwrap()
}
