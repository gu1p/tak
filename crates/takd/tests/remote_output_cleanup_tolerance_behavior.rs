use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::support;
use prost::Message;

use support::env::{EnvGuard, env_lock};
use support::remote_output::{submit_shell_task_with_outputs, test_context_with_runtime};
use support::wait_for_terminal_events::wait_for_terminal_events;
use takd::{RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn successful_remote_tasks_still_report_success_when_exec_root_cleanup_fails() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("exec-root");
    fs::create_dir_all(&exec_root_base).expect("create exec root");
    let base_mode = fs::metadata(&exec_root_base)
        .expect("exec root metadata")
        .permissions()
        .mode();

    let context = test_context_with_runtime(
        RemoteRuntimeConfig::for_tests()
            .with_explicit_remote_exec_root(exec_root_base.clone())
            .with_skip_exec_root_probe(true),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit_ack = submit_shell_task_with_outputs(
        &context,
        &store,
        "task-run-cleanup-perms",
        "mkdir -p dist && printf 'hello remote\\n' > dist/out.txt && chmod u-w ..",
        vec![tak_proto::OutputSelector {
            kind: Some(tak_proto::output_selector::Kind::Path(
                "dist/out.txt".to_string(),
            )),
        }],
    );
    assert!(submit_ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-cleanup-perms");

    let mut permissions = fs::metadata(&exec_root_base)
        .expect("updated exec root metadata")
        .permissions();
    permissions.set_mode(base_mode);
    fs::set_permissions(&exec_root_base, permissions).expect("restore exec root permissions");

    let result = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/task-run-cleanup-perms/result",
        None,
    )
    .expect("result response");
    let result =
        tak_proto::GetTaskResultResponse::decode(result.body.as_slice()).expect("decode result");
    assert!(result.success, "{result:?}");

    let output = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/task-run-cleanup-perms/outputs?path=dist/out.txt",
        None,
    )
    .expect("output response");
    assert_eq!(output.status_code, 200);
    assert_eq!(output.body, b"hello remote\n");
}
