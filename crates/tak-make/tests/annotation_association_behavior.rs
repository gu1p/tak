use tak_make::{ContainerSource, ExecutionPlacement, GoalAnnotations};

use crate::fixtures::run_source;

#[tokio::test]
async fn recipe_comments_cannot_annotate_the_next_goal() {
    let source = "first:\n\t# tak: execution=remote\nsecond:\n\t@:\n";

    let (_, request) = run_source(source, "second").await.expect("run second goal");

    assert_eq!(request.annotations, GoalAnnotations::default());
}

#[tokio::test]
async fn later_unannotated_rule_does_not_erase_goal_annotations() {
    let source = "# tak: execution=remote\n\
                  # tak: container-image=alpine:3.20\n\
                  test: first\n\
                  test: second\n";

    let (_, request) = run_source(source, "test").await.expect("run repeated goal");

    assert_eq!(
        request.annotations.placement,
        Some(ExecutionPlacement::Remote)
    );
    assert_eq!(
        request.annotations.container,
        Some(ContainerSource::Image {
            image: "alpine:3.20".to_string(),
        })
    );
}

#[tokio::test]
async fn conflicting_annotations_on_repeated_goal_are_rejected() {
    let source = "# tak: execution=local\ntest: first\n\
                  # tak: execution=remote\ntest: second\n";

    let error = run_source(source, "test")
        .await
        .expect_err("conflicting goal annotations should fail")
        .to_string();

    assert!(
        error.contains("conflicting") && error.contains("test"),
        "{error}"
    );
}
