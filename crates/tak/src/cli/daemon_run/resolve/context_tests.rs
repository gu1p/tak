use std::collections::BTreeSet;

use tak_core::v2::{
    Affinity, Session, SessionReuse, TaskContext, WorkspaceEntry, WorkspaceManifest,
};

use super::context;

#[test]
fn context_paths_filter_each_job_view_and_reinclude_explicit_paths() {
    let manifest = WorkspaceManifest::new(vec![
        WorkspaceEntry::directory("src").unwrap(),
        WorkspaceEntry::file("src/main.rs", false, 1, &"a".repeat(64)).unwrap(),
        WorkspaceEntry::directory("src/generated").unwrap(),
        WorkspaceEntry::file("src/generated/drop.rs", false, 1, &"b".repeat(64)).unwrap(),
        WorkspaceEntry::file("src/generated/keep.rs", false, 1, &"c".repeat(64)).unwrap(),
        WorkspaceEntry::file("README.md", false, 1, &"d".repeat(64)).unwrap(),
    ])
    .unwrap();
    let context = TaskContext {
        roots: vec!["src".into()],
        ignored_paths: vec!["src/generated".into()],
        use_gitignore: true,
        include: vec!["src/generated/keep.rs".into()],
    };
    let gitignored = BTreeSet::from([
        "src/generated".into(),
        "src/generated/drop.rs".into(),
        "src/generated/keep.rs".into(),
    ]);

    assert_eq!(
        context::paths(&manifest, Some(&context), &gitignored).unwrap(),
        [
            "src",
            "src/generated",
            "src/generated/keep.rs",
            "src/main.rs"
        ]
    );
    assert_eq!(
        context::paths(&manifest, None, &gitignored).unwrap(),
        ["README.md", "src", "src/main.rs"]
    );

    let unignored = TaskContext {
        roots: vec!["src".into()],
        use_gitignore: false,
        ..TaskContext::default()
    };
    assert_eq!(
        context::paths(&manifest, Some(&unignored), &gitignored)
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn explicit_task_context_overrides_the_inherited_session_context() {
    let task = TaskContext {
        roots: vec!["task".into()],
        ..TaskContext::default()
    };
    let mut session = Session::new(
        "build",
        SessionReuse::shared_workspace(1).unwrap(),
        Some(Affinity::require_same_node("build").unwrap()),
    )
    .unwrap();
    session.context = Some(TaskContext {
        roots: vec!["session".into()],
        ..TaskContext::default()
    });

    assert_eq!(
        context::effective(Some(&task), Some(&session))
            .unwrap()
            .roots,
        ["task"]
    );
    assert_eq!(
        context::effective(None, Some(&session)).unwrap().roots,
        ["session"]
    );
}
