use tak_make::{ContainerSource, ExecutionPlacement, GoalAnnotations};

use crate::fixtures::run_source;

async fn parsed_annotations(source: &str) -> GoalAnnotations {
    let (_, request) = run_source(source, "test")
        .await
        .expect("annotated literal goal should run");
    request.annotations
}

#[tokio::test]
async fn contiguous_annotations_select_remote_image_execution() {
    let annotations = parsed_annotations(
        "# tak: execution=remote\n\
         # tak: container-image=alpine:3.20\n\
         test:\n\
         \t@:\n",
    )
    .await;

    assert_eq!(
        annotations,
        GoalAnnotations {
            placement: Some(ExecutionPlacement::Remote),
            container: Some(ContainerSource::Image {
                image: "alpine:3.20".to_string(),
            }),
        }
    );
}

#[tokio::test]
async fn explicit_local_annotation_selects_local_execution() {
    let annotations = parsed_annotations("# tak: execution=local\ntest:\n\t@:\n").await;

    assert_eq!(annotations.placement, Some(ExecutionPlacement::Local));
}

#[tokio::test]
async fn contiguous_annotations_select_dockerfile_and_build_context() {
    let annotations = parsed_annotations(
        "# tak: execution=remote\n\
         # tak: container-dockerfile=docker/test.Dockerfile\n\
         # tak: container-build-context=.\n\
         test:\n\
         \t@:\n",
    )
    .await;

    assert_eq!(
        annotations,
        GoalAnnotations {
            placement: Some(ExecutionPlacement::Remote),
            container: Some(ContainerSource::Dockerfile {
                dockerfile: "docker/test.Dockerfile".to_string(),
                build_context: Some(".".to_string()),
            }),
        }
    );
}

#[tokio::test]
async fn blank_line_breaks_annotation_association() {
    let annotations = parsed_annotations(
        "# tak: execution=remote\n\
         # tak: container-image=alpine:3.20\n\
         \n\
         test:\n\
         \t@:\n",
    )
    .await;

    assert_eq!(annotations, GoalAnnotations::default());
}

#[tokio::test]
async fn ordinary_comment_breaks_annotation_association() {
    let annotations = parsed_annotations(
        "# tak: execution=remote\n\
         # build team note\n\
         test:\n\
         \t@:\n",
    )
    .await;

    assert_eq!(annotations, GoalAnnotations::default());
}
