//! Behavioral tests for core label and DAG planning contracts.

use std::collections::BTreeMap;

use tak_core::{
    label::{TaskLabel, parse_label},
    model::{
        ContainerMountDef, ContainerResourceLimitsDef, ContextManifest, CurrentStateSpec,
        IgnoreSourceSpec, PathAnchor, PathRef, RemoteRuntimeDef, build_current_state_manifest,
        normalize_container_image_reference, normalize_path_ref,
        validate_container_runtime_execution_spec,
    },
    planner::topo_sort,
};

/// Ensures relative labels are expanded using the current package namespace.
#[test]
fn parses_relative_label_using_current_package() {
    let label = parse_label(":build", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/web".to_string(),
            name: "build".to_string()
        }
    );
}

/// Ensures fully-qualified labels parse without package context dependency.
#[test]
fn parses_absolute_label() {
    let label = parse_label("//apps/api:test", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/api".to_string(),
            name: "test".to_string()
        }
    );
}

/// Ensures clean absolute labels (`package:name`) parse without `//` syntax.
#[test]
fn parses_clean_absolute_label_without_slashes() {
    let label = parse_label("apps/api:test", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/api".to_string(),
            name: "test".to_string()
        }
    );
}

/// Ensures bare task names parse as root package labels for CLI ergonomics.
#[test]
fn parses_root_label_from_bare_task_name() {
    let label = parse_label("hello", "//").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//".to_string(),
            name: "hello".to_string()
        }
    );
}

/// Ensures label display omits the internal `//` package prefix.
#[test]
fn display_omits_double_slash_prefix() {
    let nested = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };
    let root = TaskLabel {
        package: "//".to_string(),
        name: "hello".to_string(),
    };

    assert_eq!(nested.to_string(), "apps/web:test");
    assert_eq!(root.to_string(), "hello");
}

/// Ensures topological sorting places dependencies before dependents.
#[test]
fn topo_sort_returns_dependency_first_order() {
    let build = TaskLabel {
        package: "//apps/web".to_string(),
        name: "build".to_string(),
    };
    let test = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };

    let mut deps = BTreeMap::new();
    deps.insert(build.clone(), Vec::new());
    deps.insert(test.clone(), vec![build.clone()]);

    let sorted = topo_sort(&deps).expect("topo sort should succeed");
    assert_eq!(sorted, vec![build, test]);
}

/// Ensures cycle detection reports an error for cyclic dependency graphs.
#[test]
fn topo_sort_detects_cycle() {
    let a = TaskLabel {
        package: "//apps/web".to_string(),
        name: "a".to_string(),
    };
    let b = TaskLabel {
        package: "//apps/web".to_string(),
        name: "b".to_string(),
    };

    let mut deps = BTreeMap::new();
    deps.insert(a.clone(), vec![b.clone()]);
    deps.insert(b.clone(), vec![a.clone()]);

    let err = topo_sort(&deps).expect_err("should fail on cycle");
    assert!(err.to_string().contains("cycle"));
}

/// Ensures anchored path normalization is canonical across separators and `.` segments.
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

/// Ensures parent traversal escaping outside anchor boundaries is rejected.
#[test]
fn normalize_path_ref_rejects_escape_segments() {
    let err = normalize_path_ref("workspace", "../secret.txt").expect_err("must reject escape");
    assert!(
        err.to_string().contains("escapes anchor"),
        "unexpected error: {err}"
    );

    let err =
        normalize_path_ref("package", "src/../../outside.txt").expect_err("must reject escape");
    assert!(
        err.to_string().contains("escapes anchor"),
        "unexpected error: {err}"
    );
}

/// Ensures anchor references are validated before path normalization.
#[test]
fn normalize_path_ref_rejects_invalid_anchors() {
    let err = normalize_path_ref("", "src/lib.rs").expect_err("empty anchor must fail");
    assert!(
        err.to_string().contains("anchor cannot be empty"),
        "unexpected error: {err}"
    );

    let err = normalize_path_ref("repo:", "src/lib.rs").expect_err("empty repo anchor must fail");
    assert!(
        err.to_string().contains("repo anchor name cannot be empty"),
        "unexpected error: {err}"
    );

    let err = normalize_path_ref("remote", "src/lib.rs").expect_err("unsupported anchor must fail");
    assert!(
        err.to_string().contains("unsupported anchor"),
        "unexpected error: {err}"
    );
}

/// Ensures semantically equivalent input paths normalize to the same canonical path.
#[test]
fn normalize_path_ref_is_platform_stable_for_equivalent_inputs() {
    let first =
        normalize_path_ref("workspace", r"dir//subdir\.\file.txt").expect("first path valid");
    let second = normalize_path_ref("workspace", "dir/subdir/file.txt").expect("second path valid");

    assert_eq!(first, second);
}

/// Ensures semantically identical transfer inputs yield identical normalized manifests and hashes.
#[test]
fn context_manifest_hash_is_stable_for_semantically_identical_inputs() {
    let first = ContextManifest::from_paths(vec![
        normalize_path_ref("workspace", "./apps/web/src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("repo:infra", "configs/prod/deploy.yml").expect("repo path"),
    ]);

    let reordered_and_equivalent = ContextManifest::from_paths(vec![
        normalize_path_ref("repo:infra", r"configs\prod/./deploy.yml").expect("repo path"),
        normalize_path_ref("workspace", "apps//web/./src/main.rs").expect("workspace path"),
        normalize_path_ref("package", "./src/./lib.rs").expect("package path"),
        normalize_path_ref("package", "src/lib.rs").expect("duplicate package path"),
    ]);

    assert_eq!(first.entries, reordered_and_equivalent.entries);
    assert_eq!(first.hash, reordered_and_equivalent.hash);
}

/// Ensures manifest ordering is canonical and duplicate entries are removed deterministically.
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
                path: "src/lib.rs".to_string(),
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

/// Ensures manifest hash changes when the transfer set changes.
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

/// Ensures the hashing algorithm remains stable through a pinned test vector.
#[test]
fn context_manifest_hash_matches_pinned_vector() {
    let manifest = ContextManifest::from_paths(vec![
        normalize_path_ref("package", "src/lib.rs").expect("package path"),
        normalize_path_ref("repo:infra", "configs/prod/deploy.yml").expect("repo path"),
        normalize_path_ref("workspace", "apps/web/TASKS.py").expect("workspace path"),
    ]);

    assert_eq!(
        manifest.hash,
        "25905f01888000386f21a76356a0ada7a154c4369ed910138348504f10e3e7e7"
    );
}

/// Ensures transfer boundary evaluation applies `roots -> ignored -> include` deterministically.
#[test]
fn current_state_boundary_roots_then_ignored_then_include() {
    let available_files = vec![
        normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
            .expect("reinclude path"),
        normalize_path_ref("workspace", "apps/web/outside/should_not_transfer.txt")
            .expect("outside path"),
    ];

    let state = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root path")],
        ignored: vec![
            IgnoreSourceSpec::Path(
                normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored path"),
            ),
            IgnoreSourceSpec::Path(
                normalize_path_ref("workspace", "apps/web/project/ignored")
                    .expect("duplicate ignored path"),
            ),
        ],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include reinclude path"),
            normalize_path_ref("workspace", "apps/web/outside/should_not_transfer.txt")
                .expect("include outside root path"),
        ],
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

/// Ensures semantically equivalent transfer declarations produce stable context hashes.
#[test]
fn current_state_boundary_hash_is_stable_for_equivalent_state() {
    let available_files = vec![
        normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop path"),
        normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
            .expect("reinclude path"),
    ];

    let first = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root path")],
        ignored: vec![IgnoreSourceSpec::Path(
            normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored path"),
        )],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include path"),
        ],
    };

    let second = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/./project").expect("root path")],
        ignored: vec![
            IgnoreSourceSpec::Path(
                normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored path"),
            ),
            IgnoreSourceSpec::Path(
                normalize_path_ref("workspace", "apps/web/project/ignored")
                    .expect("duplicate ignored path"),
            ),
        ],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include path"),
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("duplicate include path"),
        ],
    };

    let manifest_a = build_current_state_manifest(available_files.clone(), &first);
    let manifest_b = build_current_state_manifest(available_files, &second);
    assert_eq!(manifest_a.entries, manifest_b.entries);
    assert_eq!(manifest_a.hash, manifest_b.hash);
}

/// Ensures digest-pinned image references normalize to a stable canonical form.
#[test]
fn container_image_reference_normalizes_digest_pinned_values() {
    let first = normalize_container_image_reference(
        " ghcr.io/acme/api@SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA ",
    )
    .expect("digest reference should normalize");
    let second = normalize_container_image_reference(
        "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("equivalent digest reference should normalize");

    assert_eq!(
        first.canonical,
        "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(first.canonical, second.canonical);
    assert!(first.digest_pinned);
    assert!(second.digest_pinned);
}

/// Ensures malformed digest references fail fast during normalization.
#[test]
fn container_image_reference_rejects_malformed_digests() {
    let invalid = [
        "ghcr.io/acme/api@sha256",
        "ghcr.io/acme/api@sha256:",
        "ghcr.io/acme/api@sha256:abc",
        "ghcr.io/acme/api@sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ];

    for image in invalid {
        let error = normalize_container_image_reference(image)
            .expect_err("malformed digest references must be rejected");
        assert!(
            error.to_string().contains("digest"),
            "diagnostic should identify digest issue for {image}: {error}"
        );
    }
}

/// Ensures mutable-tag references remain allowed in V1 and are marked as non-digest-pinned.
#[test]
fn container_image_reference_policy_explicitly_allows_mutable_tags() {
    let normalized =
        normalize_container_image_reference("tak/test:v1").expect("mutable tag should be allowed");

    assert_eq!(normalized.canonical, "tak/test:v1");
    assert!(!normalized.digest_pinned);
}

/// Ensures container runtime execution specs normalize valid image/command/mount/env/limits fields.
#[test]
fn container_runtime_execution_spec_normalizes_valid_fields() {
    let runtime = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some(
            " GHCR.IO/acme/api@SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA "
                .to_string(),
        ),
        command: Some(vec![" /bin/sh ".to_string(), " -lc ".to_string(), "echo hi".to_string()]),
        mounts: vec![ContainerMountDef {
            source: " ./workspace ".to_string(),
            target: " /work/./src// ".to_string(),
            read_only: true,
        }],
        env: std::collections::BTreeMap::from([
            ("APP_ENV".to_string(), "ci".to_string()),
            ("FEATURE_FLAG".to_string(), "1".to_string()),
        ]),
        resource_limits: Some(ContainerResourceLimitsDef {
            cpu_cores: Some(2.0),
            memory_mb: Some(512),
        }),
    };

    let spec = validate_container_runtime_execution_spec(&runtime)
        .expect("valid runtime config should normalize");

    assert_eq!(
        spec.image,
        "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(spec.command, vec!["/bin/sh", "-lc", "echo hi"]);
    assert_eq!(spec.mounts.len(), 1);
    assert_eq!(spec.mounts[0].source, "./workspace");
    assert_eq!(spec.mounts[0].target, "/work/src");
    assert!(spec.mounts[0].read_only);
    assert_eq!(spec.env.get("APP_ENV").map(String::as_str), Some("ci"));
    assert_eq!(
        spec.resource_limits.as_ref().and_then(|v| v.cpu_cores),
        Some(2.0)
    );
    assert_eq!(
        spec.resource_limits.as_ref().and_then(|v| v.memory_mb),
        Some(512)
    );
}

/// Ensures invalid mount/resource settings fail validation before runtime startup paths.
#[test]
fn container_runtime_execution_spec_rejects_invalid_mounts_and_limits() {
    let invalid_mount = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        command: None,
        mounts: vec![ContainerMountDef {
            source: "./workspace".to_string(),
            target: "work/src".to_string(),
            read_only: false,
        }],
        env: std::collections::BTreeMap::new(),
        resource_limits: None,
    };
    let mount_err = validate_container_runtime_execution_spec(&invalid_mount)
        .expect_err("relative mount target must fail");
    assert!(
        mount_err.to_string().contains("mount"),
        "diagnostic should identify mount issue: {mount_err}"
    );

    let invalid_limits = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        command: None,
        mounts: Vec::new(),
        env: std::collections::BTreeMap::new(),
        resource_limits: Some(ContainerResourceLimitsDef {
            cpu_cores: Some(0.0),
            memory_mb: Some(0),
        }),
    };
    let limits_err = validate_container_runtime_execution_spec(&invalid_limits)
        .expect_err("invalid cpu/memory limits must fail");
    assert!(
        limits_err.to_string().contains("cpu") || limits_err.to_string().contains("memory"),
        "diagnostic should identify resource limit issue: {limits_err}"
    );
}

/// Ensures sensitive env values are redacted from validation diagnostics.
#[test]
fn container_runtime_execution_spec_redacts_sensitive_env_values_in_errors() {
    let secret_value = "super-secret-token";
    let runtime = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        command: None,
        mounts: Vec::new(),
        env: std::collections::BTreeMap::from([(
            "SERVICE_TOKEN".to_string(),
            format!("{secret_value}\0"),
        )]),
        resource_limits: None,
    };

    let err = validate_container_runtime_execution_spec(&runtime)
        .expect_err("invalid env value should fail and redact token-like fields");
    let message = err.to_string();
    assert!(message.contains("SERVICE_TOKEN"));
    assert!(message.contains("<redacted>"));
    assert!(!message.contains(secret_value));
}
