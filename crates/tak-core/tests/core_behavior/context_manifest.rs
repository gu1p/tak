use tak_core::model::{ContextManifest, PathAnchor, PathRef, normalize_path_ref};

#[test]
fn context_manifest_hash_is_stable_for_semantically_identical_inputs() {
    let first = ContextManifest::from_paths(vec![
        normalize_path_ref("workspace", "./apps/web/src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("repo:infra", "configs/prod/deploy.yml").expect("repo path"),
    ]);
    let second = ContextManifest::from_paths(vec![
        normalize_path_ref("repo:infra", r"configs\prod/./deploy.yml").expect("repo path"),
        normalize_path_ref("workspace", "apps//web/./src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "./src/./lib.rs").expect("package path"),
        normalize_path_ref("package", "src/lib.rs").expect("duplicate package path"),
    ]);
    assert_eq!(first.entries, second.entries);
    assert_eq!(first.hash, second.hash);
}

#[test]
fn context_manifest_canonicalizes_entry_order_and_dedupes() {
    let manifest = ContextManifest::from_paths(vec![
        normalize_path_ref("workspace", "apps/web/src/main.rs").expect("workspace path"),
        normalize_path_ref("repo:infra", "configs/prod/deploy.yml").expect("repo path"),
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("package", "src/lib.rs").expect("duplicate package path"),
    ]);

    assert_eq!(
        manifest.entries,
        vec![
            PathRef {
                anchor: PathAnchor::Package,
                path: "src/lib.rs".to_string()
            },
            PathRef {
                anchor: PathAnchor::Repo("infra".to_string()),
                path: "configs/prod/deploy.yml".to_string(),
            },
            PathRef {
                anchor: PathAnchor::Workspace,
                path: "apps/web/src/main.rs".to_string(),
            },
        ]
    );
}

#[test]
fn context_manifest_hash_changes_when_transfer_set_changes() {
    let baseline = ContextManifest::from_paths(vec![
        normalize_path_ref("workspace", "apps/web/src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
    ]);
    let changed = ContextManifest::from_paths(vec![
        normalize_path_ref("workspace", "apps/web/src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("package", "src/new.rs").expect("extra package path"),
    ]);
    assert_ne!(baseline.hash, changed.hash);
}
