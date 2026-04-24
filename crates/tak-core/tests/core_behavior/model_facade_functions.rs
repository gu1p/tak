use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use tak_core::model;

#[test]
fn model_facade_reexports_root_functions() {
    let path = model::normalize_path_ref("workspace", "src/lib.rs").expect("path ref");
    let manifest = model::build_current_state_manifest(
        vec![path.clone()],
        &model::CurrentStateSpec::default(),
    );
    let image = model::normalize_container_image_reference("ghcr.io/acme/app:latest")
        .expect("image reference");
    let runtime = model::RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("ghcr.io/acme/app:latest".to_string()),
        dockerfile: None,
        build_context: None,
        command: Some(vec!["echo".to_string(), "ok".to_string()]),
        mounts: Vec::new(),
        env: BTreeMap::new(),
        resource_limits: None,
    };
    let execution =
        model::validate_container_runtime_execution_spec(&runtime).expect("execution spec");
    let workspace = model::WorkspaceSpec {
        project_id: "demo".to_string(),
        root: PathBuf::from("."),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };

    assert_eq!(path.path, "src/lib.rs");
    assert_eq!(manifest.entries, vec![path]);
    assert_eq!(image.canonical, "ghcr.io/acme/app:latest");
    assert!(matches!(
        execution.source,
        model::ContainerRuntimeSourceInputSpec::Image { ref image }
            if image == "ghcr.io/acme/app:latest"
    ));
    assert_eq!(workspace.project_id, "demo");
}
