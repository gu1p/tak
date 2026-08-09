use crate::fixtures::run_source;

#[tokio::test]
async fn invalid_global_defaults_fail_even_when_the_goal_is_valid() {
    let cases = [
        (
            "# tak: default.execution=somewhere\ntest:\n\t@:\n",
            "invalid `execution`",
        ),
        (
            "# tak: default.container-image=first\n\
             # tak: default.container-image=second\n\
             test:\n\
             \t@:\n",
            "duplicate Tak annotation `container-image`",
        ),
        (
            "# tak: default.container-image=alpine\n\
             # tak: default.container-dockerfile=Dockerfile\n\
             test:\n\
             \t@:\n",
            "mutually exclusive",
        ),
        (
            "# tak: default.container-build-context=.\ntest:\n\t@:\n",
            "requires `container-dockerfile`",
        ),
        (
            "# tak: default.unknown=value\ntest:\n\t@:\n",
            "unknown Tak annotation `unknown`",
        ),
    ];

    for (source, expected) in cases {
        let error = run_source(source, "test")
            .await
            .expect_err("invalid defaults should fail")
            .to_string();
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}
