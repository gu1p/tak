use std::fs;

use serde_json::json;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn v2_image_and_dockerfile_containers_preserve_safe_mounts_and_environment() {
    for (name, constructor) in [
        ("image", "Container.Image('alpine:3.20'"),
        (
            "dockerfile",
            "Container.Dockerfile(path('docker/Dockerfile'), path('docker')",
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("docker/cache")).unwrap();
        fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n").unwrap();
        let source = format!(
            r#"SPEC = module_spec(spec_version=2, tasks=[task("check",
  steps=[cmd("true")],
  execution=Execution.Local(container={constructor},
    mounts=[{{"source": "docker/cache", "target": "/var/cache/build/./", "read_only": True}}],
    env={{"APP_ENV": "ci", "ORDER": "runtime"}})))])
SPEC
"#
        );
        fs::write(temp.path().join("TASKS.py"), source).unwrap();

        let root = inspect_authored_root_module(temp.path(), &LoadOptions::default())
            .unwrap_or_else(|error| panic!("{name} container failed: {error:#}"));
        let module = serde_json::to_value(root.module).unwrap();
        let runtime = &module["tasks"][0]["execution"]["local"]["runtime"];
        assert_eq!(runtime["env"], json!({"APP_ENV": "ci", "ORDER": "runtime"}));
        assert_eq!(
            runtime["mounts"],
            json!([{
                "source": "docker/cache",
                "target": "/var/cache/build",
                "read_only": true,
            }])
        );
    }
}

#[test]
fn removed_container_command_points_to_task_steps() {
    let error = load_error("Container.Image('alpine:3.20', command=['sh', '-c', 'echo obsolete'])");
    assert!(error.contains("Container `command` was removed"), "{error}");
    assert!(error.contains("use task steps"), "{error}");
}

#[test]
fn v2_mount_sources_cannot_name_daemon_or_worker_host_paths() {
    for source in [
        "/etc",
        "../outside",
        "cache/../../outside",
        "//../outside",
        "///etc",
    ] {
        let error = load_error(&format!(
            "Container.Image('alpine:3.20', mounts=[{{'source': '{source}', 'target': '/mnt', 'read_only': False}}])"
        ));
        assert!(error.contains("workspace-relative"), "{source}: {error}");
        assert!(error.contains("daemon-owned"), "{source}: {error}");
    }
}

fn load_error(container: &str) -> String {
    let temp = tempfile::tempdir().unwrap();
    let source = format!(
        "SPEC=module_spec(spec_version=2, tasks=[task('check', execution=Execution.Local(container={container}))])\nSPEC\n"
    );
    fs::write(temp.path().join("TASKS.py"), source).unwrap();
    inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .unwrap_err()
        .to_string()
}
