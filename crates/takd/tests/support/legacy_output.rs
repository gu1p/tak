use std::fs;
use std::path::{Path, PathBuf};

use takd::{SubmitAttemptStore, SubmitRegistration};

pub fn seed(
    store: &SubmitAttemptStore,
    task_run_id: &str,
    execution_root_base: &Path,
    output_path: &str,
    content: &[u8],
) -> PathBuf {
    let registration = store
        .register_submit_with_execution_root_base(
            task_run_id,
            Some(1),
            "//:legacy-history",
            None,
            "builder-a",
            execution_root_base,
        )
        .expect("register legacy history");
    let key = match registration {
        SubmitRegistration::Created { idempotency_key }
        | SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    };
    let artifact_root = execution_root_base
        .parent()
        .expect("execution root parent")
        .join("takd-remote-artifacts")
        .join(key.replace(':', "_"));
    let output = artifact_root.join(output_path);
    fs::create_dir_all(output.parent().expect("output parent")).expect("artifact directories");
    fs::write(&output, content).expect("legacy output");
    let result = serde_json::json!({
        "success": true,
        "outputs": [{"path": output_path, "digest": "legacy", "size": content.len()}]
    });
    store
        .set_result_payload(&key, &result.to_string())
        .expect("persist legacy result");
    artifact_root
}
