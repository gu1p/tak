#![allow(clippy::await_holding_lock)]

use tak_core::model::RemoteTransportKind;
use tak_exec::{NoMatchingRemoteError, RemoteCandidateRejection, RunOptions, run_tasks};

mod support;

use support::{
    EnvGuard, env_lock, prepare_workspace, remote_builder_spec, remote_task_spec, shell_step,
    write_enabled_remote_mismatches,
};

#[tokio::test]
async fn no_matching_remote_error_reports_each_enabled_remote_rejection() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let (_temp, workspace_root, config_root) = prepare_workspace(&mut env);
    write_enabled_remote_mismatches(&config_root);

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_requires_build_pool",
        vec![shell_step("echo should-not-run")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("run should fail without a matching remote");
    let diagnostic = err
        .downcast_ref::<NoMatchingRemoteError>()
        .expect("expected a structured no-matching-remote error");

    assert_eq!(
        diagnostic.task_label,
        format!("{}:{}", label.package, label.name)
    );
    assert_eq!(diagnostic.required.pool.as_deref(), Some("build"));
    assert_eq!(diagnostic.required.required_tags, vec!["builder"]);
    assert_eq!(diagnostic.required.required_capabilities, vec!["linux"]);
    assert_eq!(diagnostic.required.transport_kind, RemoteTransportKind::Tor);
    assert_eq!(diagnostic.enabled_remotes.len(), 3);

    let default_remote = diagnostic
        .enabled_remotes
        .iter()
        .find(|remote| remote.node_id == "builder-default")
        .expect("builder-default should be listed");
    assert_eq!(
        default_remote.rejection_reasons,
        vec![RemoteCandidateRejection::PoolMismatch {
            required: "build".into(),
            available: vec!["default".into()],
        }]
    );

    let direct_remote = diagnostic
        .enabled_remotes
        .iter()
        .find(|remote| remote.node_id == "builder-direct")
        .expect("builder-direct should be listed");
    assert_eq!(
        direct_remote.rejection_reasons,
        vec![RemoteCandidateRejection::TransportMismatch {
            required: RemoteTransportKind::Tor,
            available: "direct".into(),
        }]
    );

    let macos_remote = diagnostic
        .enabled_remotes
        .iter()
        .find(|remote| remote.node_id == "builder-macos")
        .expect("builder-macos should be listed");
    assert_eq!(
        macos_remote.rejection_reasons,
        vec![
            RemoteCandidateRejection::MissingTags {
                missing: vec!["builder".into()],
                available: vec!["runner".into()],
            },
            RemoteCandidateRejection::MissingCapabilities {
                missing: vec!["linux".into()],
                available: vec!["macos".into()],
            },
        ]
    );
}
