//! Protocol-v2 submission contracts for `tak exec --remote` overrides.

use serde_json::json;

#[path = "exec_remote_override_contract/support.rs"]
mod support;

#[test]
fn exec_submits_remote_container_image_override() {
    let captured = support::run(&[
        "exec",
        "--remote",
        "--container-image",
        "alpine:3.20",
        "--",
        "true",
    ]);

    assert!(captured.output.status.success(), "{}", captured.stderr());
    assert_eq!(
        captured.runtime(),
        &json!({"kind": "container", "source": {
            "kind": "image", "image": "alpine:3.20"
        }})
    );
    assert_eq!(captured.candidate()["node_id"], "worker-a");
}

#[test]
fn exec_submits_remote_dockerfile_override() {
    let captured = support::run(&[
        "exec",
        "--remote",
        "--container-dockerfile",
        "docker/Dockerfile",
        "--",
        "true",
    ]);

    assert!(captured.output.status.success(), "{}", captured.stderr());
    assert_eq!(
        captured.runtime(),
        &json!({"kind": "container", "source": {
            "kind": "dockerfile", "dockerfile": "docker/Dockerfile",
            "build_context": "docker"
        }})
    );
}

#[test]
fn exec_warns_that_remote_container_flag_is_redundant() {
    let captured = support::run(&[
        "exec",
        "--remote",
        "--container",
        "--container-image",
        "alpine:3.20",
        "--",
        "true",
    ]);

    assert!(captured.output.status.success(), "{}", captured.stderr());
    assert!(
        captured.stderr().contains(
            "warning: --container is redundant with --remote; remote execution already implies a container"
        ),
        "{}",
        captured.stderr()
    );
}
