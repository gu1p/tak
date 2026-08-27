use super::{
    BollardError,
    execution_wait::{container_removal_result, docker_wait_result_exit_code},
};

#[test]
fn docker_wait_result_exit_code_accepts_zero_status() {
    let result = docker_wait_result_exit_code(Ok(bollard::models::ContainerWaitResponse {
        status_code: 0,
        error: None,
    }))
    .expect("zero exit status should be accepted");

    assert_eq!(result, 0);
}

#[test]
fn docker_wait_result_exit_code_treats_empty_wait_error_as_task_exit() {
    let result = docker_wait_result_exit_code(Err(BollardError::DockerContainerWaitError {
        error: String::new(),
        code: 1,
    }))
    .expect("empty wait error should preserve task exit status");

    assert_eq!(result, 1);
}

#[test]
fn docker_wait_result_exit_code_preserves_wait_error_message() {
    let err = docker_wait_result_exit_code(Err(BollardError::DockerContainerWaitError {
        error: "context canceled".to_string(),
        code: 1,
    }))
    .expect_err("daemon-side wait error should surface as infra failure");

    assert_eq!(
        err.to_string(),
        "infra error: container lifecycle runtime failed: docker wait failed (status 1): context canceled"
    );
}

#[test]
fn container_cleanup_only_ignores_an_already_removed_container() {
    let missing = BollardError::DockerResponseServerError {
        status_code: 404,
        message: "no such container".to_string(),
    };
    container_removal_result(Err(missing), "gone").expect("404 is idempotent cleanup");

    let failure = BollardError::DockerResponseServerError {
        status_code: 500,
        message: "engine refused removal".to_string(),
    };
    let error = container_removal_result(Err(failure), "still-running")
        .expect_err("unconfirmed teardown must be visible");
    let detail = format!("{error:#}");
    assert!(detail.contains("still-running"));
    assert!(detail.contains("engine refused removal"));
}
