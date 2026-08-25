use crate::fixtures::run_source;

async fn rejects(source: &str) -> String {
    run_source(source, "all")
        .await
        .expect_err("invalid parallel group")
        .to_string()
}

#[tokio::test]
async fn parallel_members_must_be_unique_direct_phony_prerequisites() {
    for (source, expected) in [
        (
            ".PHONY: all a\na:\n# tak: parallel=a\nall: a\n",
            "at least two",
        ),
        (
            ".PHONY: all a\na:\n# tak: parallel=a,a\nall: a\n",
            "duplicate",
        ),
        (".PHONY: all\na:\n# tak: parallel=a,b\nall: a b\n", ".PHONY"),
        (
            ".PHONY: all a b\na:\nb:\n# tak: parallel=a,b\nall: a\n",
            "direct prerequisite",
        ),
        (
            ".PHONY: all a b c\na: c\nb:\nc:\n# tak: parallel=a,b\nall: c\n",
            "direct prerequisite",
        ),
    ] {
        let error = rejects(source).await;
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[tokio::test]
async fn cycles_and_parallel_defaults_are_rejected() {
    let default_error =
        rejects("# tak: default.parallel=a,b\n.PHONY: all a b\na:\nb:\nall: a b\n").await;
    assert!(
        default_error.contains("default.parallel"),
        "{default_error}"
    );

    let cycle_error = rejects(
        ".PHONY: all a b\n# tak: parallel=all,b\na: all b\nb:\n# tak: parallel=a,b\nall: a b\n",
    )
    .await;
    assert!(cycle_error.contains("cycle"), "{cycle_error}");
}
