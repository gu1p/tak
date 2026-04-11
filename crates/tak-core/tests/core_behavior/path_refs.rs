use tak_core::model::{PathAnchor, normalize_path_ref};

#[test]
fn normalize_path_ref_canonicalizes_anchors_and_segments() {
    let workspace = normalize_path_ref("workspace", "./apps//web/./TASKS.py")
        .expect("workspace path should normalize");
    assert_eq!(workspace.anchor, PathAnchor::Workspace);
    assert_eq!(workspace.path, "apps/web/TASKS.py");

    let package =
        normalize_path_ref("package", r".\src\.\lib.rs").expect("package path should normalize");
    assert_eq!(package.anchor, PathAnchor::Package);
    assert_eq!(package.path, "src/lib.rs");

    let repo = normalize_path_ref("repo:infra", r"configs\\prod/./deploy.yml")
        .expect("repo path should normalize");
    assert_eq!(repo.anchor, PathAnchor::Repo("infra".to_string()));
    assert_eq!(repo.path, "configs/prod/deploy.yml");
}

#[test]
fn normalize_path_ref_rejects_escape_segments() {
    let err = normalize_path_ref("workspace", "../secret.txt").expect_err("must reject escape");
    assert!(err.to_string().contains("escapes anchor"));

    let err =
        normalize_path_ref("package", "src/../../outside.txt").expect_err("must reject escape");
    assert!(err.to_string().contains("escapes anchor"));
}

#[test]
fn normalize_path_ref_rejects_invalid_anchors() {
    assert!(normalize_path_ref("", "src/lib.rs").is_err());
    assert!(normalize_path_ref("repo:", "src/lib.rs").is_err());
    assert!(normalize_path_ref("remote", "src/lib.rs").is_err());
}

#[test]
fn normalize_path_ref_is_platform_stable_for_equivalent_inputs() {
    let first =
        normalize_path_ref("workspace", r"dir//subdir\.\file.txt").expect("first path valid");
    let second = normalize_path_ref("workspace", "dir/subdir/file.txt").expect("second path valid");
    assert_eq!(first, second);
}
