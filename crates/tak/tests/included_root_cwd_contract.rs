use std::fs;
use std::path::{Path, PathBuf};

const MODULES: [&str; 19] = [
    "examples/small/03_relative_vs_absolute_labels/apps/web/TASKS.py",
    "examples/medium/18_multi_package_monorepo/apps/api/TASKS.py",
    "examples/medium/18_multi_package_monorepo/apps/web/TASKS.py",
    "examples/medium/18_multi_package_monorepo/libs/common/TASKS.py",
    "examples/large/21_recursive_enterprise_monorepo/apps/portal/TASKS.py",
    "examples/large/21_recursive_enterprise_monorepo/platform/auth/TASKS.py",
    "examples/large/21_recursive_enterprise_monorepo/platform/billing/TASKS.py",
    "examples/large/22_polyglot_pipeline_build_test_release/services/js/TASKS.py",
    "examples/large/22_polyglot_pipeline_build_test_release/services/python/TASKS.py",
    "examples/large/22_polyglot_pipeline_build_test_release/services/rust/TASKS.py",
    "examples/large/23_contention_heavy_daemon_coordination/apps/a/TASKS.py",
    "examples/large/23_contention_heavy_daemon_coordination/apps/b/TASKS.py",
    "examples/large/23_contention_heavy_daemon_coordination/apps/c/TASKS.py",
    "examples/large/24_full_feature_matrix_end_to_end/apps/qa/TASKS.py",
    "examples/large/24_full_feature_matrix_end_to_end/libs/common/TASKS.py",
    "examples/large/25_remote_direct_build_and_artifact_roundtrip/services/api/TASKS.py",
    "examples/large/27_hybrid_local_remote_test_suite_success/apps/web/TASKS.py",
    "examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/apps/web/TASKS.py",
    "examples/large/29_remote_any_transport_container_log_storm/apps/logstorm/TASKS.py",
];

#[test]
fn included_steps_that_share_root_artifacts_use_explicit_workspace_cwd() {
    let root = repo_root();
    for relative in MODULES {
        let body = fs::read_to_string(root.join(relative)).expect("read included TASKS.py");
        let steps = body.matches("cmd(").count() + body.matches("script(").count();
        assert_eq!(body.matches("cwd=\"//\"").count(), steps, "{relative}");
    }
}

#[test]
fn included_script_steps_reference_root_scripts_explicitly() {
    let root = repo_root();
    for (relative, script) in [
        (
            "examples/large/22_polyglot_pipeline_build_test_release/services/python/TASKS.py",
            "//scripts/release.sh",
        ),
        (
            "examples/large/24_full_feature_matrix_end_to_end/apps/qa/TASKS.py",
            "//scripts/matrix_release.sh",
        ),
    ] {
        let body = fs::read_to_string(root.join(relative)).expect("read included TASKS.py");
        assert!(body.contains(&format!("script(\"{script}\"")), "{relative}");
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}
