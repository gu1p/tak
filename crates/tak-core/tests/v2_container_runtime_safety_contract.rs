use crate::v2_resolved_run_support::sample_run;
use serde_json::json;
use tak_core::v2::{ContainerSource, TaskRuntime};

#[test]
fn runtime_paths_images_and_timeouts_are_canonical_and_safe() {
    for runtime in [
        TaskRuntime::container(ContainerSource::Image { image: " ".into() }),
        TaskRuntime::container(ContainerSource::Dockerfile {
            dockerfile: "../Dockerfile".into(),
            build_context: ".".into(),
        }),
        TaskRuntime::container(ContainerSource::Dockerfile {
            dockerfile: "docker/Dockerfile".into(),
            build_context: "other".into(),
        }),
    ] {
        let mut run = sample_run();
        run.tasks[0].runtime = Some(runtime);
        assert!(run.validate().is_err());
    }
    let mut run = sample_run();
    run.tasks[0].timeout_s = Some(0);
    assert!(run.validate().is_err());
}

#[test]
fn container_mounts_and_environment_are_validated_at_the_resolved_run_boundary() {
    let mut run = sample_run();
    run.tasks[0].runtime = Some(runtime(
        json!([{"source": "cache/input", "target": "/var/cache/input", "read_only": true}]),
        json!({"APP_ENV": "ci"}),
    ));
    run.validate().unwrap();

    for mounts in [
        json!([{"source": "/etc", "target": "/mnt", "read_only": true}]),
        json!([{"source": "../outside", "target": "/mnt", "read_only": true}]),
        json!([{"source": "cache", "target": "relative", "read_only": true}]),
        json!([{"source": "cache", "target": "/mnt/../escape", "read_only": true}]),
    ] {
        let mut run = sample_run();
        run.tasks[0].runtime = Some(runtime(mounts, json!({})));
        assert!(run.validate().is_err());
    }
    for env in [
        json!({"bad-name": "value"}),
        json!({"TAK_RUNTIME": "spoof"}),
    ] {
        let mut run = sample_run();
        run.tasks[0].runtime = Some(runtime(json!([]), env));
        assert!(run.validate().is_err());
    }
}

fn runtime(mounts: serde_json::Value, env: serde_json::Value) -> TaskRuntime {
    serde_json::from_value(json!({
        "kind": "container",
        "source": {"kind": "image", "image": "alpine:3.20"},
        "mounts": mounts,
        "env": env,
    }))
    .unwrap()
}
