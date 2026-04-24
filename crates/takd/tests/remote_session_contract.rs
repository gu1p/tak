use takd::{RemoteRuntimeConfig, SubmitAttemptStore};

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::remote_output::test_context_with_runtime;
use support::remote_session::{assert_success, session, submit_session_task};
use support::wait_for_terminal_events::wait_for_terminal_events;

#[test]
fn share_workspace_session_preserves_remote_workspace_between_tasks() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("exec-root");

    let context = test_context_with_runtime(
        RemoteRuntimeConfig::for_tests()
            .with_explicit_remote_exec_root(exec_root_base)
            .with_skip_exec_root_probe(true),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    submit_session_task(
        &context,
        &store,
        "task-run-build",
        "mkdir -p .session && printf 'cached\\n' > .session/build.txt",
        session("run-1-rust", "share_workspace", Vec::new()),
    );
    wait_for_terminal_events(&context, &store, "task-run-build");
    assert_success(&context, &store, "task-run-build");

    submit_session_task(
        &context,
        &store,
        "task-run-test",
        "test -f .session/build.txt",
        session("run-1-rust", "share_workspace", Vec::new()),
    );
    wait_for_terminal_events(&context, &store, "task-run-test");
    assert_success(&context, &store, "task-run-test");
}
