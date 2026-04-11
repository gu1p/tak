use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_proto::{runtime_spec, step};

use super::submit_payload_test_support::{
    direct_target, encoded_workspace, task_with_steps_and_needs, workspace,
};
use super::*;

#[test]
fn build_remote_submit_payload_includes_runtime_steps_and_declared_needs() {
    let target = direct_target(Some(RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "ghcr.io/acme/web:latest".into(),
        },
    }));
    let payload = build_remote_submit_payload(
        &target,
        "task-run-1",
        7,
        &task_with_steps_and_needs(),
        &workspace(&encoded_workspace()),
    )
    .expect("submit payload");

    assert_eq!(payload.task_run_id, "task-run-1");
    assert_eq!(payload.attempt, 7);
    assert_eq!(payload.workspace_zip, b"zip-bytes");
    assert_eq!(payload.timeout_s, Some(30));
    assert_eq!(payload.task_label, "apps/web:build");
    assert_eq!(
        payload
            .needs
            .iter()
            .map(|need| (need.name.clone(), need.scope.clone(), need.scope_key.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("cpu".into(), "machine".into(), None),
            ("network".into(), "user".into(), Some("builder".into())),
            ("deploy".into(), "project".into(), Some("apps/web".into())),
            ("disk".into(), "worktree".into(), None),
        ]
    );
    match payload.runtime.expect("runtime").kind.expect("runtime kind") {
        runtime_spec::Kind::Container(container) => {
            assert_eq!(container.image.as_deref(), Some("ghcr.io/acme/web:latest"));
            assert_eq!(container.dockerfile, None);
            assert_eq!(container.build_context, None);
        }
    }
    match payload.steps[0].kind.as_ref().expect("cmd step") {
        step::Kind::Cmd(cmd) => assert_eq!(cmd.argv, vec!["cargo", "test"]),
        other => panic!("expected cmd step, got {other:?}"),
    }
    match payload.steps[1].kind.as_ref().expect("script step") {
        step::Kind::Script(script) => assert_eq!(script.path, "scripts/build.sh"),
        other => panic!("expected script step, got {other:?}"),
    }
}
