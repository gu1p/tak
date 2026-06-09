use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use super::helpers::{context_under, recording_node};
use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, env_lock, remote_builder_spec,
    remote_task_spec_with_context, shell_step,
};

#[tokio::test]
async fn distinct_workspace_content_uploads_separately() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(workspace.join("alpha")).expect("alpha dir");
    std::fs::create_dir_all(workspace.join("beta")).expect("beta dir");
    std::fs::write(workspace.join("alpha/a.txt"), b"alpha-contents").expect("a.txt");
    std::fs::write(workspace.join("beta/b.txt"), b"beta-contents").expect("b.txt");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let node = RecordingRemoteServer::spawn_success("builder-a", events.clone());
    recording_node(&config, &node);

    // Two tasks whose contexts select different files → different content hashes → no reuse.
    let (mut spec, alpha) = remote_task_spec_with_context(
        &workspace,
        "alpha-task",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
        context_under("alpha"),
    );
    let (beta_spec, beta) = remote_task_spec_with_context(
        &workspace,
        "beta-task",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
        context_under("beta"),
    );
    for (label, task) in beta_spec.tasks {
        spec.tasks.insert(label, task);
    }

    run_tasks(
        &spec,
        &[alpha, beta],
        &RunOptions {
            jobs: 1,
            ..RunOptions::default()
        },
    )
    .await
    .expect("both remote tasks should run");

    assert_eq!(
        events.upload_begin_count(),
        2,
        "tasks with different workspace content must each upload"
    );
}
