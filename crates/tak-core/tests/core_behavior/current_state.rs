use tak_core::model::{
    CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, PathAnchor, PathRef,
    build_current_state_manifest, normalize_path_ref,
};

#[test]
fn context_manifest_hash_matches_pinned_vector() {
    let manifest = tak_core::model::ContextManifest::from_paths(vec![
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("repo:infra", "configs/prod/deploy.yml").expect("repo path"),
        normalize_path_ref("workspace", "apps/web/TASKS.py").expect("workspace path"),
    ]);
    assert_eq!(
        manifest.hash,
        "25905f01888000386f21a76356a0ada7a154c4369ed910138348504f10e3e7e7"
    );
}

#[test]
fn current_state_boundary_roots_then_ignored_then_include() {
    let available_files = vec![
        normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
            .expect("reinclude"),
        normalize_path_ref("workspace", "apps/web/outside/should_not_transfer.txt")
            .expect("outside"),
    ];
    let state = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root")],
        ignored: vec![IgnoreSourceSpec::Path(
            normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored"),
        )],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include"),
        ],
        origin: CurrentStateOrigin::Explicit,
    };
    let manifest = build_current_state_manifest(available_files, &state);
    assert_eq!(
        manifest.entries,
        vec![
            PathRef {
                anchor: PathAnchor::Workspace,
                path: "apps/web/project/ignored/reinclude.txt".to_string(),
            },
            PathRef {
                anchor: PathAnchor::Workspace,
                path: "apps/web/project/keep.txt".to_string(),
            },
        ]
    );
}

#[test]
fn current_state_boundary_hash_is_stable_for_equivalent_state() {
    let available_files = vec![
        normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
            .expect("reinclude"),
    ];
    let first = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root")],
        ignored: vec![IgnoreSourceSpec::Path(
            normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored"),
        )],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include"),
        ],
        origin: CurrentStateOrigin::Explicit,
    };
    let second = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/./project").expect("root")],
        ignored: first.ignored.clone(),
        include: first.include.clone(),
        origin: CurrentStateOrigin::Explicit,
    };
    let manifest_a = build_current_state_manifest(available_files.clone(), &first);
    let manifest_b = build_current_state_manifest(available_files, &second);
    assert_eq!(manifest_a.entries, manifest_b.entries);
    assert_eq!(manifest_a.hash, manifest_b.hash);
}
