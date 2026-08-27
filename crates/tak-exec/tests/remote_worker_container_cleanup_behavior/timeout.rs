use std::time::Duration;

use super::*;

#[tokio::test]
async fn unconfirmed_removal_does_not_block_the_terminal_timeout_result() {
    let (temp, daemon, workspace_root, mut spec, mut locked_env) = cleanup_case();
    configure_real_docker_env(temp.path(), daemon.socket_path(), locked_env.env_mut());
    daemon.fail_container_removal("removal refused");
    spec.timeout_s = Some(0);
    let observer = Arc::new(CollectingObserver::default());

    let result = tokio::time::timeout(
        Duration::from_millis(500),
        execute_remote_worker_steps_with_output_and_cancellation(
            &workspace_root,
            &spec,
            Some(observer),
            &tak_exec::RunCancellation::default(),
        ),
    )
    .await
    .expect("terminal timeout result must not wait for follow logs")
    .expect("task timeout should return a terminal result");

    assert!(!result.success);
    assert_eq!(result.exit_code, None);
    assert_eq!(removed_container_ids(&daemon), vec!["container-123"]);
}
