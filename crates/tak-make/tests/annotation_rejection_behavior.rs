use crate::fixtures::run_source;

async fn rejection(source: &str, goal: &str) -> String {
    run_source(source, goal)
        .await
        .expect_err("invalid Makefile annotation should be rejected")
        .to_string()
}

#[tokio::test]
async fn annotated_multi_target_rule_is_rejected() {
    let error = rejection(
        "# tak: execution=remote\n\
         first second:\n\
         \t@:\n",
        "first",
    )
    .await;

    assert!(error.contains("literal single-target"), "{error}");
}

#[tokio::test]
async fn annotated_pattern_rule_is_rejected() {
    let error = rejection(
        "# tak: execution=remote\n\
         %.o: %.c\n\
         \t@:\n",
        "%.o",
    )
    .await;

    assert!(error.contains("literal single-target"), "{error}");
}

#[tokio::test]
async fn annotated_double_colon_rule_is_rejected() {
    let error = rejection(
        "# tak: execution=remote\n\
         test::\n\
         \t@:\n",
        "test",
    )
    .await;

    assert!(error.contains("literal single-target"), "{error}");
}

#[tokio::test]
async fn annotated_static_pattern_rule_is_rejected() {
    let error = rejection(
        "# tak: execution=remote\n\
         test: %.o: %.c\n\
         \t@:\n",
        "test",
    )
    .await;

    assert!(error.contains("literal single-target"), "{error}");
}

#[tokio::test]
async fn image_and_dockerfile_annotations_are_rejected_together() {
    let error = rejection(
        "# tak: container-image=alpine:3.20\n\
         # tak: container-dockerfile=docker/test.Dockerfile\n\
         test:\n\
         \t@:\n",
        "test",
    )
    .await;

    assert!(
        error.contains("container-image") && error.contains("container-dockerfile"),
        "{error}"
    );
}

#[tokio::test]
async fn missing_requested_goal_is_rejected() {
    let error = rejection("other:\n\t@:\n", "test").await;

    assert!(
        error.contains("test") && error.contains("not found"),
        "{error}"
    );
}
