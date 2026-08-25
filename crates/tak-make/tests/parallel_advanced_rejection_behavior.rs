use crate::fixtures::run_source;

async fn rejects(source: &str) -> String {
    run_source(source, "all")
        .await
        .expect_err("unsafe parallel graph")
        .to_string()
}

#[tokio::test]
async fn dynamic_and_continued_parallel_prerequisites_are_rejected() {
    let dynamic = ".PHONY: all a b\na:\nb:\n# tak: parallel=a,b\nall: $(PARTS)\n";
    let continued = concat!(
        ".PHONY: all a b\na:\nb:\n# tak: parallel=a,b\n",
        "all: a \\\n",
        "b\n"
    );
    for source in [dynamic, continued] {
        let error = rejects(source).await;
        assert!(error.contains("literal prerequisites"), "{error}");
    }
}

#[tokio::test]
async fn shared_targets_with_conflicting_inherited_execution_are_rejected() {
    let error = rejects(
        ".PHONY: all left right shared left-only right-only\n\
         shared:\nleft-only:\nright-only:\n\
         # tak: execution=remote\n# tak: parallel=shared,left-only\n\
         left: shared left-only\n\
         # tak: execution=local\n# tak: parallel=shared,right-only\n\
         right: shared right-only\n\
         # tak: parallel=left,right\nall: left right\n",
    )
    .await;

    assert!(error.contains("conflicting Tak annotations"), "{error}");
}

#[tokio::test]
async fn invalid_parallel_groups_elsewhere_are_not_ignored() {
    let error = run_source(
        ".PHONY: test all\na:\nb:\n# tak: parallel=a,b\nall: a b\ntest:\n",
        "test",
    )
    .await
    .expect_err("invalid metadata elsewhere should fail closed")
    .to_string();

    assert!(error.contains(".PHONY"), "{error}");
}
