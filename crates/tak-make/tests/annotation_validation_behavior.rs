use crate::fixtures::run_source;

#[tokio::test]
async fn invalid_annotation_forms_fail_clearly() {
    for (source, expected) in [
        ("# tak: execution\ntest:\n", "key=value"),
        ("# tak: =remote\ntest:\n", "key=value"),
        ("# tak: execution=\ntest:\n", "key=value"),
        ("# tak: mystery=value\ntest:\n", "unknown"),
        (
            "# tak: execution=local\n# tak: execution=remote\ntest:\n",
            "duplicate",
        ),
        ("# tak: execution=nearby\ntest:\n", "local` or `remote"),
        (
            "# tak: parallel-output=quiet\ntest:\n",
            "expected `live` or `grouped`",
        ),
        (
            "# tak: container-build-context=.\ntest:\n",
            "requires `container-dockerfile`",
        ),
        (
            "# tak: container-image=alpine:3.20\n\
             # tak: container-build-context=.\ntest:\n",
            "requires `container-dockerfile`",
        ),
    ] {
        let error = run_source(source, "test")
            .await
            .expect_err("invalid annotation should fail")
            .to_string();
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[tokio::test]
async fn annotated_expanded_target_is_rejected() {
    let error = run_source("# tak: execution=remote\n$(GOAL):\n", "test")
        .await
        .expect_err("expanded target should fail")
        .to_string();

    assert!(error.contains("literal single-target"), "{error}");
}

#[tokio::test]
async fn annotated_target_specific_assignment_is_rejected() {
    for declaration in [
        "test: CFLAGS = -O2",
        "test: CFLAGS := -O2",
        "test: CFLAGS ::= -O2",
        "test: CFLAGS :::= -O2",
        "test: CFLAGS ?= -O2",
        "test: CFLAGS += -O2",
        "test: CFLAGS != printf=-O2",
        "test: export CFLAGS = -O2",
        "test: private CFLAGS = -O2",
        "test: override CFLAGS = -O2",
        "test: unexport CFLAGS = -O2",
        "test: private override CFLAGS = -O2",
    ] {
        let source = format!("# tak: execution=remote\n{declaration}\n");
        let error = run_source(&source, "test")
            .await
            .expect_err("target-specific assignment should fail")
            .to_string();
        assert!(error.contains("literal single-target"), "{error}");
    }
}

#[tokio::test]
async fn invalid_annotations_on_other_goals_are_not_silently_ignored() {
    let source = "# tak: mystery=value\nother:\ntest:\n";

    let error = run_source(source, "test")
        .await
        .expect_err("invalid metadata elsewhere should fail closed")
        .to_string();

    assert!(error.contains("unknown"), "{error}");
}
