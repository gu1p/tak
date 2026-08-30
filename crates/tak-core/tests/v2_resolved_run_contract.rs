use tak_core::v2::{EnvironmentValue, RunSubmission, WorkspaceEntry, WorkspaceManifest};

use super::v2_resolved_run_support::sample_run;

#[test]
fn canonical_workspace_fingerprint_includes_kind_mode_target_size_and_content() {
    let entries = vec![
        WorkspaceEntry::file("bin/check", true, 3, &"a".repeat(64)).unwrap(),
        WorkspaceEntry::directory("bin").unwrap(),
        WorkspaceEntry::symlink("current", "bin/check").unwrap(),
    ];
    let forward = WorkspaceManifest::new(entries.clone()).unwrap();
    let reverse = WorkspaceManifest::new(entries.into_iter().rev()).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.entries[0].path, "bin");
    assert_eq!(forward.fingerprint.len(), 64);
}

#[test]
fn canonical_workspace_rejects_absolute_escaping_and_duplicate_entries() {
    assert!(WorkspaceEntry::directory("a//b").is_err());
    for target in ["/etc/passwd", "../escape", "dir/../../escape"] {
        let error = WorkspaceEntry::symlink("link", target).unwrap_err();
        assert!(error.to_string().contains("symlink"), "{target}: {error}");
    }
    let duplicate = WorkspaceEntry::directory("same").unwrap();
    let error = WorkspaceManifest::new(vec![duplicate.clone(), duplicate]).unwrap_err();
    assert!(error.to_string().contains("duplicate"), "{error}");
}

#[test]
fn canonical_workspace_rejects_hierarchy_and_symlink_chain_escapes() {
    let directory = WorkspaceEntry::directory("inside").unwrap();
    let parent = WorkspaceEntry::directory("a").unwrap();
    let pivot = WorkspaceEntry::symlink("a/pivot", "../inside").unwrap();
    let escaping = WorkspaceEntry::symlink("a/link", "pivot/../..").unwrap();
    assert!(
        WorkspaceManifest::new([directory, parent, pivot, escaping]).is_err(),
        "a target must not traverse a declared symlink before applying `..`"
    );

    let ancestor = WorkspaceEntry::symlink("tree", "inside").unwrap();
    let descendant = WorkspaceEntry::directory("tree/child").unwrap();
    assert!(WorkspaceManifest::new([ancestor, descendant]).is_err());
}

#[test]
fn resolved_run_validates_task_job_edges_and_context_references() {
    let mut run = sample_run();
    run.tasks[0].job_id = "missing-job".into();
    assert!(
        run.validate()
            .unwrap_err()
            .to_string()
            .contains("missing-job")
    );

    let mut run = sample_run();
    run.jobs[0].context_manifest.paths = vec!["missing.txt".into()];
    assert!(
        run.validate()
            .unwrap_err()
            .to_string()
            .contains("missing.txt")
    );

    let mut run = sample_run();
    run.targets.push("//:missing".into());
    assert!(
        run.validate()
            .unwrap_err()
            .to_string()
            .contains("//:missing")
    );
}

#[test]
fn submission_digest_is_stable_and_secret_debug_is_redacted() {
    let value = EnvironmentValue::new("TOKEN", "swordfish").unwrap();
    let left = RunSubmission::new("idem-1", sample_run(), vec![value.clone()]).unwrap();
    let right = RunSubmission::new("idem-1", sample_run(), vec![value]).unwrap();

    assert_eq!(left.request_digest(), right.request_digest());
    let debug = format!("{left:?}");
    assert!(!debug.contains("swordfish"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
}
